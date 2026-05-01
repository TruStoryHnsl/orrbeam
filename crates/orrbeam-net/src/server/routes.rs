//! Endpoint handlers for the orrbeam control-plane HTTPS server.
//!
//! # Route map
//!
//! ## Unauthenticated
//! - `GET  /v1/hello`                        — exchange public identity
//! - `POST /v1/mutual-trust-request`         — initiate TOFU handshake
//! - `GET  /v1/mutual-trust-request/{id}`    — poll TOFU status
//!
//! ## Authenticated (require Ed25519 signature)
//! - `GET  /v1/status`                       — query Sunshine / Moonlight status
//! - `POST /v1/sunshine/start`               — start the Sunshine service
//! - `POST /v1/sunshine/stop`                — stop the Sunshine service
//! - `POST /v1/pair/accept`                  — submit a Moonlight pairing PIN
//! - `GET  /v1/peers`                        — list trusted peers

#![warn(missing_docs)]

use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json,
    extract::{ConnectInfo, Extension, Path, State},
};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use orrbeam_core::wire::HelloPayload;
use orrbeam_platform::ServiceStatus;

use super::errors::ControlError;
use super::middleware::PeerContext;
use super::{ControlState, MutualTrustStatus, PendingMutualTrust};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response body for `POST /v1/mutual-trust-request`.
#[derive(Debug, Serialize)]
pub struct MutualTrustResponse {
    /// Current status string: always `"pending"` on initial creation.
    pub status: String,
    /// ISO 8601 timestamp at which this request expires.
    pub pending_until: String,
}

/// Response body for `GET /v1/mutual-trust-request/{id}`.
#[derive(Debug, Serialize)]
pub struct MutualTrustPollResponse {
    /// Current status: `"pending"`, `"approved"`, or `"rejected"`.
    pub status: String,
    /// Receiver's public hello, present only when the request is approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_hello: Option<HelloPayload>,
}

/// Response body for `GET /v1/status`.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Sunshine service information.
    pub sunshine: SunshineStatus,
    /// Moonlight service information.
    pub moonlight: MoonlightStatus,
}

/// Sunshine-specific status fragment.
#[derive(Debug, Serialize)]
pub struct SunshineStatus {
    /// Whether Sunshine is currently running.
    pub running: bool,
    /// Whether Sunshine is installed on this node.
    pub installed: bool,
    /// Version string, if detectable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Moonlight-specific status fragment.
#[derive(Debug, Serialize)]
pub struct MoonlightStatus {
    /// Whether Moonlight is currently running.
    pub running: bool,
    /// Whether Moonlight is installed on this node.
    pub installed: bool,
    /// Version string, if detectable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Response body for `POST /v1/sunshine/start`.
#[derive(Debug, Serialize)]
pub struct StartResponse {
    /// Whether Sunshine is (now) running.
    pub started: bool,
    /// Version string, if detectable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Response body for `POST /v1/sunshine/stop`.
#[derive(Debug, Serialize)]
pub struct StopResponse {
    /// Whether Sunshine was stopped successfully.
    pub stopped: bool,
}

/// Response body for `POST /v1/pair/accept`.
#[derive(Debug, Serialize)]
pub struct PairAcceptResponse {
    /// Whether the PIN was accepted by Sunshine.
    pub accepted: bool,
}

/// A single sanitized peer record returned by `GET /v1/peers`.
#[derive(Debug, Serialize)]
pub struct PeerSummary {
    /// Human-readable name.
    pub name: String,
    /// Ed25519 key fingerprint.
    pub ed25519_fingerprint: String,
    /// ISO 8601 timestamp of when this peer was last seen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
    /// Address (host or IP).
    pub address: String,
    /// Tags associated with this peer.
    pub tags: Vec<String>,
}

/// Response body for `GET /v1/peers`.
#[derive(Debug, Serialize)]
pub struct PeersListResponse {
    /// All trusted peers on this node, sanitized (no raw keys).
    pub peers: Vec<PeerSummary>,
}

// ---------------------------------------------------------------------------
// Request body types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/mutual-trust-request`.
#[derive(Debug, Deserialize)]
pub struct MutualTrustBody {
    /// The requesting node's public identity.
    pub initiator: HelloPayload,
    /// Optional human-readable note from the requester.
    pub note: Option<String>,
    /// Client-generated unique ID for this trust request.
    pub request_id: Uuid,
}

/// Request body for `POST /v1/pair/accept`.
#[derive(Debug, Deserialize)]
pub struct PairAcceptBody {
    /// The 4-digit Moonlight pairing PIN.
    pub pin: String,
    /// The name of the Moonlight client as displayed in Sunshine.
    pub client_name: String,
}

// ---------------------------------------------------------------------------
// Unauthenticated handlers
// ---------------------------------------------------------------------------

/// `GET /v1/hello` — exchange public identity and capability advertisement.
///
/// Always returns 200. No authentication required — this endpoint is used
/// during the initial TOFU handshake and node discovery.
pub async fn hello(State(state): State<Arc<ControlState>>) -> Json<HelloPayload> {
    let config = state.config.read().await;
    let platform_info = state.platform.info();

    let sunshine_available = state
        .platform
        .sunshine_status(&config)
        .map(|s| s.status == ServiceStatus::Running || s.status == ServiceStatus::Installed)
        .unwrap_or(false);

    let moonlight_available = state
        .platform
        .moonlight_status(&config)
        .map(|s| s.status == ServiceStatus::Running || s.status == ServiceStatus::Installed)
        .unwrap_or(false);

    let public_key_b64 = STANDARD_NO_PAD.encode(state.identity.public_key().as_bytes());

    Json(HelloPayload {
        node_name: config.node_name.clone(),
        ed25519_fingerprint: state.identity.fingerprint(),
        ed25519_public_key_b64: public_key_b64,
        cert_sha256: state.tls.cert_sha256_hex.clone(),
        control_port: {
            let cfg = &*config;
            // The control port is not directly in Config yet; default to 47782.
            let _ = cfg;
            47782_u16
        },
        sunshine_available,
        moonlight_available,
        os: platform_info.os.clone(),
        version: orrbeam_core::wire::PROTOCOL_VERSION.to_string(),
    })
}

/// `POST /v1/mutual-trust-request` — initiate a TOFU mutual-trust handshake.
///
/// No authentication required. The receiver emits an event so the UI can
/// prompt the user to approve or reject. Returns 202 with a `pending_until`
/// field; the caller should poll `GET /v1/mutual-trust-request/{id}` for the
/// outcome.
pub async fn mutual_trust_request(
    State(state): State<Arc<ControlState>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    Json(body): Json<MutualTrustBody>,
) -> Result<Json<MutualTrustResponse>, ControlError> {
    let now = time::OffsetDateTime::now_utc();
    let expires_at = now + Duration::from_secs(60);

    let client_ip = peer_addr.ip();

    // §19.9 Per-IP rate-limit: max 3 TOFU requests per IP per 60-second window.
    {
        let mut ip_map = state.ip_tofu_attempts.write().await;
        let attempts = ip_map.entry(client_ip).or_default();
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(60);
        attempts.retain(|&t| t > cutoff);
        if attempts.len() >= 3 {
            return Err(ControlError::RateLimited);
        }
        attempts.push(std::time::Instant::now());
    }

    // §19.9 Global pending cap: max 1 pending request at a time.
    {
        let store = state.pending_mutual_trust.read().await;
        for entry in store.values() {
            let age = (now - entry.created_at).unsigned_abs();
            if entry.status == MutualTrustStatus::Pending && age < Duration::from_secs(60) {
                return Err(ControlError::TofuPending);
            }
        }
    }

    let initiator_clone = body.initiator.clone();

    {
        let mut store = state.pending_mutual_trust.write().await;
        store.insert(
            body.request_id,
            PendingMutualTrust {
                initiator: body.initiator,
                note: body.note,
                created_at: now,
                status: MutualTrustStatus::Pending,
                receiver_hello: None,
            },
        );
    }

    // Emit inbound trust request event for the frontend.
    state
        .event_emitter
        .emit(
            "peers:mutual-trust-inbound",
            serde_json::to_value(&initiator_clone).unwrap_or(serde_json::Value::Null),
        )
        .await;

    let pending_until = expires_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| expires_at.unix_timestamp().to_string());

    Ok(Json(MutualTrustResponse {
        status: "pending".to_string(),
        pending_until,
    }))
}

/// `GET /v1/mutual-trust-request/{id}` — poll the status of a TOFU request.
///
/// Returns 410 Gone if the request is not found or has expired. Returns
/// 200 with `status = "approved"` and `receiver_hello` when the local user
/// approved the request.
pub async fn mutual_trust_poll(
    State(state): State<Arc<ControlState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<MutualTrustPollResponse>, ControlError> {
    let now = time::OffsetDateTime::now_utc();
    let store = state.pending_mutual_trust.read().await;

    let entry = store.get(&id).ok_or(ControlError::TofuExpired)?;

    let age = (now - entry.created_at).unsigned_abs();
    if entry.status == MutualTrustStatus::Pending && age >= Duration::from_secs(60) {
        return Err(ControlError::TofuExpired);
    }

    let status_str = match entry.status {
        MutualTrustStatus::Pending => "pending",
        MutualTrustStatus::Approved => "approved",
        MutualTrustStatus::Rejected => "rejected",
    };

    Ok(Json(MutualTrustPollResponse {
        status: status_str.to_string(),
        receiver_hello: entry.receiver_hello.clone(),
    }))
}

// ---------------------------------------------------------------------------
// Authenticated handlers
// ---------------------------------------------------------------------------

/// `GET /v1/status` — query Sunshine and Moonlight service status.
///
/// Requires: `can_query_status` permission.
pub async fn status(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
) -> Result<Json<StatusResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_query_status {
        return Err(ControlError::Forbidden(
            "peer lacks can_query_status permission".into(),
        ));
    }

    let config = state.config.read().await;

    let sunshine_info = state
        .platform
        .sunshine_status(&config)
        .map_err(|e| ControlError::ServiceUnavailable(format!("sunshine_status failed: {e}")))?;

    let moonlight_info = state
        .platform
        .moonlight_status(&config)
        .map_err(|e| ControlError::ServiceUnavailable(format!("moonlight_status failed: {e}")))?;

    // Touch last_seen for the peer.
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    Ok(Json(StatusResponse {
        sunshine: SunshineStatus {
            running: sunshine_info.status == ServiceStatus::Running,
            installed: sunshine_info.status != ServiceStatus::NotInstalled,
            version: sunshine_info.version,
        },
        moonlight: MoonlightStatus {
            running: moonlight_info.status == ServiceStatus::Running,
            installed: moonlight_info.status != ServiceStatus::NotInstalled,
            version: moonlight_info.version,
        },
    }))
}

/// `POST /v1/sunshine/start` — start the Sunshine service.
///
/// If Sunshine is already running the call is idempotent (returns 200 with
/// `started: true` without re-launching). Requires: `can_start_sunshine`.
pub async fn sunshine_start(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
) -> Result<Json<StartResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_start_sunshine {
        return Err(ControlError::Forbidden(
            "peer lacks can_start_sunshine permission".into(),
        ));
    }

    let config = state.config.read().await;

    // Check current status — if already running, short-circuit.
    let info = state
        .platform
        .sunshine_status(&config)
        .map_err(|e| ControlError::ServiceUnavailable(format!("sunshine_status: {e}")))?;

    if info.status == ServiceStatus::Running {
        // Touch last_seen.
        {
            let mut store = state.peers.write().await;
            store.touch_last_seen(&peer_ctx.peer.name);
        }
        tracing::info!(peer = %peer_ctx.peer.name, "sunshine already running — idempotent start");
        return Ok(Json(StartResponse {
            started: true,
            version: info.version,
        }));
    }

    state
        .platform
        .start_sunshine(&config)
        .map_err(|e| ControlError::ServiceUnavailable(format!("start_sunshine: {e}")))?;

    // Re-query to get the updated version.
    let updated = state.platform.sunshine_status(&config).ok();

    // Touch last_seen.
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    tracing::info!(peer = %peer_ctx.peer.name, "sunshine started");

    Ok(Json(StartResponse {
        started: true,
        version: updated.and_then(|i| i.version),
    }))
}

/// `POST /v1/sunshine/stop` — stop the Sunshine service.
///
/// Requires: `can_stop_sunshine`.
pub async fn sunshine_stop(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
) -> Result<Json<StopResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_stop_sunshine {
        return Err(ControlError::Forbidden(
            "peer lacks can_stop_sunshine permission".into(),
        ));
    }

    state
        .platform
        .stop_sunshine()
        .map_err(|e| ControlError::ServiceUnavailable(format!("stop_sunshine: {e}")))?;

    // Touch last_seen.
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    tracing::info!(peer = %peer_ctx.peer.name, "sunshine stopped");

    Ok(Json(StopResponse { stopped: true }))
}

/// `POST /v1/pair/accept` — submit a Moonlight pairing PIN to local Sunshine.
///
/// The PIN is submitted to `https://127.0.0.1:47990/api/pin` and retried up
/// to 15 times with 1-second intervals. The actual PIN value is **never logged**.
/// Requires: `can_submit_pin`.
pub async fn pair_accept(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
    Json(body): Json<PairAcceptBody>,
) -> Result<Json<PairAcceptResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_submit_pin {
        return Err(ControlError::Forbidden(
            "peer lacks can_submit_pin permission".into(),
        ));
    }

    tracing::info!(
        peer = %peer_ctx.peer.name,
        client_name = %body.client_name,
        "PIN <redacted> submitted for pairing"
    );

    let config = state.config.read().await;
    let username = config.sunshine_username.clone();
    let password = config.sunshine_password.clone();
    drop(config); // Release the lock before the async call.

    orrbeam_core::sunshine_api::submit_pin_local(
        &username,
        &password,
        &body.pin,
        &body.client_name,
        15,
    )
    .await
    .map_err(|e| {
        use orrbeam_core::sunshine_api::SunshineApiError;
        match e {
            SunshineApiError::Unreachable => ControlError::SunshineUnreachable,
            SunshineApiError::PinRejected => ControlError::PinRejected,
            SunshineApiError::NoCredentials => {
                ControlError::Internal("Sunshine credentials not configured".into())
            }
            SunshineApiError::Http(msg) => ControlError::Internal(msg),
        }
    })?;

    // Touch last_seen.
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    Ok(Json(PairAcceptResponse { accepted: true }))
}

// ---------------------------------------------------------------------------
// Shared-control types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/shared-control/join`.
#[derive(Debug, Deserialize)]
pub struct SharedControlJoinBody {
    /// Display name of the joining participant (1–64 characters).
    pub participant_name: String,
    /// Requested slot index (0–3); the actual assigned slot may differ.
    pub slot_index: u8,
}

/// Response body for `POST /v1/shared-control/join`.
#[derive(Debug, Serialize)]
pub struct SharedControlJoinResponse {
    /// The slot index actually assigned by the host.
    pub slot_index: u8,
    /// Best-effort path to the virtual input device on the host (Linux only).
    pub device_path: String,
}

/// `POST /v1/shared-control/join` — add a remote participant to the active shared-control session.
///
/// Requires: `can_start_sunshine` permission (reuses the existing bit).
/// The host must have an active shared-control session (started via the Tauri
/// `start_shared_control` command). Returns 503 if no session is active.
pub async fn shared_control_join(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
    Json(body): Json<SharedControlJoinBody>,
) -> Result<Json<SharedControlJoinResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_start_sunshine {
        return Err(ControlError::Forbidden(
            "peer lacks can_start_sunshine permission".into(),
        ));
    }

    // Validate inputs (commercial scope: slot 0–3, name 1–64 chars).
    if body.participant_name.is_empty() || body.participant_name.len() > 64 {
        return Err(ControlError::InvalidBody(
            "participant_name must be 1–64 characters".into(),
        ));
    }
    if body.slot_index > 3 {
        return Err(ControlError::InvalidBody("slot_index must be 0–3".into()));
    }

    let assigned_slot = {
        let mut guard = state
            .shared_control
            .lock()
            .map_err(|_| ControlError::Internal("shared_control lock poisoned".into()))?;
        let session = guard
            .as_mut()
            .ok_or(ControlError::SharedControlUnavailable)?;
        session
            .add_participant(body.participant_name.clone())
            .map_err(|e| ControlError::ServiceUnavailable(format!("add_participant failed: {e}")))?
    };

    // Touch last_seen for the requesting peer.
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    tracing::info!(
        peer = %peer_ctx.peer.name,
        participant = %body.participant_name,
        assigned_slot,
        "shared-control participant joined"
    );

    Ok(Json(SharedControlJoinResponse {
        slot_index: assigned_slot,
        device_path: format!("/dev/input/event{assigned_slot}"),
    }))
}

/// `GET /v1/peers` — list trusted peers (sanitized, no raw keys).
///
/// Requires: `can_list_peers`.
pub async fn peers_list(
    State(state): State<Arc<ControlState>>,
    Extension(peer_ctx): Extension<PeerContext>,
) -> Result<Json<PeersListResponse>, ControlError> {
    if !peer_ctx.peer.permissions.can_list_peers {
        return Err(ControlError::Forbidden(
            "peer lacks can_list_peers permission".into(),
        ));
    }

    let store = state.peers.read().await;
    let peers: Vec<PeerSummary> = store
        .list()
        .into_iter()
        .map(|p| PeerSummary {
            name: p.name.clone(),
            ed25519_fingerprint: p.ed25519_fingerprint.clone(),
            last_seen_at: p.last_seen_at.and_then(|ts| {
                ts.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            }),
            address: p.address.clone(),
            tags: p.tags.clone(),
        })
        .collect();

    // Touch last_seen for the querying peer.
    drop(store);
    {
        let mut store = state.peers.write().await;
        store.touch_last_seen(&peer_ctx.peer.name);
    }

    Ok(Json(PeersListResponse { peers }))
}
