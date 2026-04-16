//! HTTPS control-plane server for the orrbeam mesh.
//!
//! This module implements the bidirectional control plane used by orrbeam nodes
//! to authenticate each other with Ed25519 signatures and orchestrate
//! Sunshine / Moonlight operations over TLS.
//!
//! # Architecture
//!
//! - **[`ControlState`]** — shared application state threaded through all
//!   handlers via `Arc<ControlState>`.
//! - **[`serve()`]** — starts the axum-server TLS listener and blocks until
//!   the `shutdown` [`CancellationToken`] is cancelled.
//! - **[`build_router()`]** — assembles the axum [`Router`] with authenticated
//!   and unauthenticated route groups.
//! - **[`EventEmitter`]** — a `Send + Sync` trait that decouples the net crate
//!   from Tauri so events can be forwarded to the frontend without introducing a
//!   compile-time dependency on `tauri`.
//!
//! # Sub-modules
//!
//! - [`errors`] — [`ControlError`] enum → HTTP status + JSON body
//! - [`middleware`] — [`require_signed`] signature-verification middleware
//! - [`nonce`] — [`NonceCache`] replay-attack prevention
//! - [`routes`] — all endpoint handlers

#![warn(missing_docs)]

pub mod errors;
pub mod middleware;
pub mod nonce;
pub mod routes;

pub use errors::ControlError;
pub use middleware::PeerContext;
pub use nonce::NonceCache;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::routing::{get, post};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// EventEmitter trait
// ---------------------------------------------------------------------------

/// Abstraction that allows `orrbeam-net` to emit events to the application
/// layer without depending on Tauri directly.
///
/// The Tauri application provides a concrete implementation that calls
/// `app_handle.emit(...)`. Tests and headless builds use [`NoopEmitter`].
#[async_trait::async_trait]
pub trait EventEmitter: Send + Sync {
    /// Emit a named event with a JSON payload.
    ///
    /// Implementations should not block. Errors should be logged internally
    /// rather than propagated — event delivery is best-effort.
    async fn emit(&self, topic: &str, payload: serde_json::Value);
}

/// A no-op [`EventEmitter`] that silently discards all events.
///
/// Used in unit tests and any context where Tauri is not available.
pub struct NoopEmitter;

#[async_trait::async_trait]
impl EventEmitter for NoopEmitter {
    async fn emit(&self, _topic: &str, _payload: serde_json::Value) {}
}

// ---------------------------------------------------------------------------
// PendingMutualTrust
// ---------------------------------------------------------------------------

/// Status of an in-flight mutual-trust (TOFU) request.
#[derive(Debug, Clone, PartialEq)]
pub enum MutualTrustStatus {
    /// The request has been received but the local user has not yet responded.
    Pending,
    /// The local user approved the mutual-trust request.
    Approved,
    /// The local user rejected the mutual-trust request.
    Rejected,
}

/// An in-flight mutual-trust (TOFU) request waiting for local user approval.
#[derive(Debug, Clone)]
pub struct PendingMutualTrust {
    /// Public identity of the node that initiated the trust request.
    pub initiator: orrbeam_core::wire::HelloPayload,
    /// Optional note from the initiator.
    pub note: Option<String>,
    /// When this request was received.
    pub created_at: time::OffsetDateTime,
    /// Current disposition of the request.
    pub status: MutualTrustStatus,
    /// This node's hello payload, populated when the request is approved.
    pub receiver_hello: Option<orrbeam_core::wire::HelloPayload>,
}

// ---------------------------------------------------------------------------
// ControlState
// ---------------------------------------------------------------------------

/// Shared application state for the control-plane server.
///
/// All fields are cheaply cloneable via `Arc` / `Arc<RwLock<…>>`. The
/// `Arc<ControlState>` is passed as axum `State` to every handler.
pub struct ControlState {
    /// This node's Ed25519 identity (used to build `/v1/hello` responses).
    pub identity: Arc<orrbeam_core::identity::Identity>,
    /// This node's TLS certificate identity (fingerprint exposed in hello).
    pub tls: Arc<orrbeam_core::tls::TlsIdentity>,
    /// Application configuration (node name, credentials, discovery settings).
    pub config: Arc<RwLock<orrbeam_core::config::Config>>,
    /// Trusted peer store (mutable because `touch_last_seen` updates it).
    pub peers: Arc<RwLock<orrbeam_core::peers::TrustedPeerStore>>,
    /// Nonce cache for replay-attack prevention.
    pub nonces: Arc<NonceCache>,
    /// In-flight TOFU mutual-trust requests keyed by request UUID.
    pub pending_mutual_trust: Arc<RwLock<HashMap<uuid::Uuid, PendingMutualTrust>>>,
    /// Platform abstraction for Sunshine / Moonlight process management.
    pub platform: Arc<dyn orrbeam_platform::Platform + Send + Sync>,
    /// Active shared-control session, if any.
    ///
    /// Shared with the Tauri `AppState` via the same `Arc` so that Tauri
    /// commands (`start_shared_control`, etc.) and the HTTP server join
    /// endpoint both operate on the same live session.
    pub shared_control: Arc<Mutex<Option<Box<dyn orrbeam_platform::shared_control::SharedControlSession + Send + Sync>>>>,
    /// Event emitter for forwarding control-plane events to the UI layer.
    pub event_emitter: Arc<dyn EventEmitter>,
    /// Cancellation token; cancel this to initiate a graceful shutdown.
    pub shutdown: CancellationToken,
    /// Per-IP timestamps of recent TOFU requests, for rate-limiting (§19.9).
    ///
    /// Enforces max 3 requests per IP per 60-second window on the
    /// `POST /v1/mutual-trust-request` endpoint.
    pub ip_tofu_attempts: Arc<RwLock<HashMap<IpAddr, Vec<Instant>>>>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the axum [`Router`] for the control-plane server.
///
/// Unauthenticated routes (`/v1/hello`, `/v1/mutual-trust-request`) are
/// exposed without signature checking. All other routes under `/v1` require
/// a valid Ed25519 signature header set and are wrapped with
/// [`middleware::require_signed`].
pub fn build_router(state: Arc<ControlState>) -> axum::Router {
    use axum::middleware::from_fn_with_state;
    use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

    // Authenticated sub-router — all routes here require a valid signature.
    let auth_routes = axum::Router::new()
        .route("/status", get(routes::status))
        .route("/sunshine/start", post(routes::sunshine_start))
        .route("/sunshine/stop", post(routes::sunshine_stop))
        .route("/pair/accept", post(routes::pair_accept))
        .route("/peers", get(routes::peers_list))
        .route("/shared-control/join", post(routes::shared_control_join))
        .layer(from_fn_with_state(
            state.clone(),
            middleware::require_signed,
        ));

    axum::Router::new()
        // Unauthenticated endpoints.
        .route("/v1/hello", get(routes::hello))
        .route(
            "/v1/mutual-trust-request",
            post(routes::mutual_trust_request),
        )
        .route(
            "/v1/mutual-trust-request/{id}",
            get(routes::mutual_trust_poll),
        )
        // Mount the authenticated group at /v1 — routes inside use short paths.
        .nest("/v1", auth_routes)
        // Global middleware layers (applied outermost, i.e. last to execute).
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// serve()
// ---------------------------------------------------------------------------

/// Start the TLS control-plane server and block until shutdown.
///
/// Binds a TLS listener at `bind` using the certificate from `state.tls`,
/// serves the router built by [`build_router`], and shuts down gracefully
/// (3-second drain) when `state.shutdown` is cancelled.
///
/// # Errors
///
/// Returns an error if the TLS configuration is invalid, the port cannot be
/// bound, or the underlying `axum_server` returns an I/O error.
pub async fn serve(
    state: Arc<ControlState>,
    bind: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tls_config = state.tls.rustls_server_config()?;
    let router = build_router(state.clone());
    let handle = axum_server::Handle::new();

    // Graceful shutdown: cancel the token → tell axum-server to drain.
    let shutdown_handle = handle.clone();
    let shutdown_token = state.shutdown.clone();
    tokio::spawn(async move {
        shutdown_token.cancelled().await;
        tracing::info!("control server shutting down gracefully");
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(3)));
    });

    tracing::info!(%bind, "control server listening");

    axum_server::bind_rustls(
        bind,
        axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(tls_config)),
    )
    .handle(handle)
    .serve(router.into_make_service_with_connect_info::<std::net::SocketAddr>())
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// NoopEmitter must compile as a valid EventEmitter and emit without
    /// panicking.
    #[tokio::test]
    async fn noop_emitter_does_not_panic() {
        let emitter = NoopEmitter;
        emitter
            .emit("test:event", serde_json::json!({"key": "value"}))
            .await;
    }

    /// MutualTrustStatus equality is required for the pending-check logic.
    #[test]
    fn mutual_trust_status_eq() {
        assert_eq!(MutualTrustStatus::Pending, MutualTrustStatus::Pending);
        assert_ne!(MutualTrustStatus::Pending, MutualTrustStatus::Approved);
        assert_ne!(MutualTrustStatus::Approved, MutualTrustStatus::Rejected);
    }

    /// build_router compiles and produces a router that can be converted to a
    /// MakeService (i.e. the type system is satisfied).
    #[test]
    fn build_router_compiles() {
        // We need a ControlState to build the router.  Use a minimal stub.
        use std::sync::Arc;
        use tokio::sync::RwLock;
        use tokio_util::sync::CancellationToken;

        // A minimal Platform implementation for tests only.
        struct StubPlatform;
        impl orrbeam_platform::Platform for StubPlatform {
            fn info(&self) -> orrbeam_platform::PlatformInfo {
                orrbeam_platform::PlatformInfo {
                    os: "test".into(),
                    os_version: None,
                    display_server: None,
                    hostname: "test".into(),
                }
            }
            fn sunshine_status(
                &self,
                _: &orrbeam_core::config::Config,
            ) -> Result<orrbeam_platform::ServiceInfo, orrbeam_platform::PlatformError> {
                Ok(orrbeam_platform::ServiceInfo {
                    name: "sunshine".into(),
                    status: orrbeam_platform::ServiceStatus::NotInstalled,
                    version: None,
                    path: None,
                })
            }
            fn moonlight_status(
                &self,
                _: &orrbeam_core::config::Config,
            ) -> Result<orrbeam_platform::ServiceInfo, orrbeam_platform::PlatformError> {
                Ok(orrbeam_platform::ServiceInfo {
                    name: "moonlight".into(),
                    status: orrbeam_platform::ServiceStatus::NotInstalled,
                    version: None,
                    path: None,
                })
            }
            fn start_sunshine(
                &self,
                _: &orrbeam_core::config::Config,
            ) -> Result<(), orrbeam_platform::PlatformError> {
                Ok(())
            }
            fn stop_sunshine(&self) -> Result<(), orrbeam_platform::PlatformError> {
                Ok(())
            }
            fn start_moonlight(
                &self,
                _: &orrbeam_core::config::Config,
                _: &str,
                _: &str,
                _: bool,
                _: Option<&str>,
            ) -> Result<(), orrbeam_platform::PlatformError> {
                Ok(())
            }
            fn stop_moonlight(&self) -> Result<(), orrbeam_platform::PlatformError> {
                Ok(())
            }
            fn monitors(
                &self,
            ) -> Result<Vec<orrbeam_platform::MonitorInfo>, orrbeam_platform::PlatformError> {
                Ok(vec![])
            }
            fn gpu_info(
                &self,
            ) -> Result<orrbeam_platform::GpuInfo, orrbeam_platform::PlatformError> {
                Ok(orrbeam_platform::GpuInfo {
                    name: "stub".into(),
                    encoder: "stub".into(),
                    driver: None,
                })
            }
            fn pair_moonlight(
                &self,
                _: &orrbeam_core::config::Config,
                _: &str,
                _: &str,
            ) -> Result<(), orrbeam_platform::PlatformError> {
                Ok(())
            }
        }

        let identity =
            Arc::new(orrbeam_core::identity::Identity::generate().expect("identity"));

        // Build a TlsIdentity with a temp dir.
        let tmp = tempfile::TempDir::new().unwrap();
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp.path()) };
        let tls = Arc::new(
            orrbeam_core::tls::TlsIdentity::load_or_create(&identity, "test-node")
                .expect("tls identity"),
        );

        let state = Arc::new(ControlState {
            identity,
            tls,
            config: Arc::new(RwLock::new(orrbeam_core::config::Config::default())),
            peers: Arc::new(RwLock::new(
                orrbeam_core::peers::TrustedPeerStore::default(),
            )),
            nonces: NonceCache::new(),
            pending_mutual_trust: Arc::new(RwLock::new(HashMap::new())),
            platform: Arc::new(StubPlatform),
            shared_control: Arc::new(std::sync::Mutex::new(None)),
            event_emitter: Arc::new(NoopEmitter),
            shutdown: CancellationToken::new(),
            ip_tofu_attempts: Arc::new(RwLock::new(HashMap::new())),
        });

        // This call exercises the full router construction — if it compiles and
        // runs without panicking the router is valid.
        let _router = build_router(state);
    }
}
