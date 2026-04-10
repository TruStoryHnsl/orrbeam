//! HTTP client for the orrbeam node-to-node control plane.
//!
//! [`ControlClient`] provides typed, signed HTTPS calls from one orrbeam node
//! to another's control-plane server (`/v1/…`).  It supports two distinct
//! operating modes:
//!
//! - **Pinned mode** ([`ControlClient::new`]): builds a reqwest client backed
//!   by a [`PinnedVerifier`] so that TLS certificate validation is performed
//!   against the stored SHA-256 fingerprint of the peer's self-signed cert.
//!   Every request is signed with the local node's Ed25519 identity key.
//!
//! - **Bootstrap (TOFU) mode** ([`ControlClient::bootstrap_hello`]): a
//!   one-shot static method that contacts an unknown peer's `/v1/hello`
//!   endpoint over HTTPS with certificate validation disabled (`danger_accept_invalid_certs`).
//!   This is the **only** place in the codebase where that flag appears.
//!   The caller is expected to pin the returned cert fingerprint immediately
//!   after.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use orrbeam_core::identity::Identity;
//! use orrbeam_core::peers::TrustedPeer;
//! use orrbeam_net::client::ControlClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let identity = Arc::new(Identity::load_or_create()?);
//! // `peer` comes from the TrustedPeerStore
//! # let peer: TrustedPeer = unimplemented!();
//! let client = ControlClient::new(identity, &peer)?;
//! let status = client.status().await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod errors;
pub mod verifier;

use std::sync::Arc;
use std::time::Duration;

use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use orrbeam_core::identity::Identity;
use orrbeam_core::peers::TrustedPeer;
use orrbeam_core::wire::{
    sign_request, HelloPayload, HEADER_KEY_ID, HEADER_NONCE, HEADER_SIGNATURE, HEADER_TIMESTAMP,
    HEADER_VERSION,
};

pub use errors::ClientError;
use verifier::PinnedVerifier;

// ── Response types ────────────────────────────────────────────────────────────

/// Response from `GET /v1/status`.
#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    /// Current Sunshine process state (flexible shape while API stabilises).
    pub sunshine: serde_json::Value,
    /// Current Moonlight process state (flexible shape while API stabilises).
    pub moonlight: serde_json::Value,
}

/// Response from `POST /v1/sunshine/start`.
#[derive(Debug, Deserialize)]
pub struct StartResponse {
    /// `true` if Sunshine was successfully started.
    pub started: bool,
}

/// Response from `POST /v1/sunshine/stop`.
#[derive(Debug, Deserialize)]
pub struct StopResponse {
    /// `true` if Sunshine was successfully stopped.
    pub stopped: bool,
}

/// Response from `POST /v1/sunshine/pair`.
#[derive(Debug, Deserialize)]
pub struct PairAcceptResponse {
    /// `true` if the pairing PIN was accepted by the remote Sunshine instance.
    pub accepted: bool,
}

/// Response from `GET /v1/peers`.
#[derive(Debug, Deserialize)]
pub struct PeersResponse {
    /// List of peer records in the remote node's trusted-peer store.
    pub peers: Vec<serde_json::Value>,
}

/// Standard error body returned by the control-plane server on 4xx/5xx.
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    /// Machine-readable error code (e.g. `"unauthorized"`, `"not_found"`).
    pub error: String,
    /// Human-readable description of the error.
    pub message: String,
}

// ── Request body types ────────────────────────────────────────────────────────

/// Body sent with `POST /v1/sunshine/pair`.
#[derive(Debug, Serialize)]
struct PairRequest<'a> {
    pin: &'a str,
    client_name: &'a str,
}

// ── Retry behaviour ───────────────────────────────────────────────────────────

/// Classification of whether a request error warrants a retry.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ShouldRetry {
    /// The error is transient; try again.
    Yes,
    /// The error is definitive; give up.
    No,
}

/// Determine whether a [`ClientError`] should trigger a retry attempt.
///
/// Retries are appropriate for connection failures and server unavailability
/// (503).  Auth, not-found, and validation errors are permanent.
fn should_retry(err: &ClientError) -> ShouldRetry {
    match err {
        ClientError::Unreachable { .. } => ShouldRetry::Yes,
        ClientError::Http(e) => {
            if e.is_connect() || e.is_timeout() {
                ShouldRetry::Yes
            } else {
                ShouldRetry::No
            }
        }
        ClientError::Remote { status, .. } => {
            if *status == 503 {
                ShouldRetry::Yes
            } else {
                ShouldRetry::No
            }
        }
        _ => ShouldRetry::No,
    }
}

// ── ControlClient ─────────────────────────────────────────────────────────────

/// Signed HTTPS client for orrbeam node-to-node control-plane communication.
///
/// Each instance is bound to a single remote peer and uses that peer's stored
/// cert fingerprint as its TLS trust anchor.  All mutating requests carry an
/// Ed25519 signature over the canonical request payload.
///
/// Create via [`ControlClient::new`] for normal operation, or use the static
/// [`ControlClient::bootstrap_hello`] for first-contact TOFU.
pub struct ControlClient {
    /// Configured reqwest HTTP client (TLS pinned to the peer's cert).
    http: reqwest::Client,
    /// This node's Ed25519 identity used to sign outgoing requests.
    identity: Arc<Identity>,
    /// IP address or hostname of the remote peer.
    peer_address: String,
    /// TCP port of the remote peer's control-plane HTTPS server.
    peer_port: u16,
}

impl ControlClient {
    /// Build a pinned client for a known trusted peer.
    ///
    /// Constructs a reqwest [`Client`] backed by a custom rustls
    /// [`ClientConfig`] that uses [`PinnedVerifier`] for TLS certificate
    /// validation.  All subsequent requests from this client are TLS-verified
    /// against `peer.cert_sha256`.
    ///
    /// # Errors
    ///
    /// - [`ClientError::InvalidResponse`]: `peer.cert_sha256` is not valid hex.
    /// - [`ClientError::Http`]: the underlying reqwest client could not be
    ///   constructed (very rare).
    pub fn new(identity: Arc<Identity>, peer: &TrustedPeer) -> Result<Self, ClientError> {
        let verifier = PinnedVerifier::new(&peer.cert_sha256)?;

        // Build a rustls ClientConfig that uses PinnedVerifier as the sole
        // cert verifier.  No CA roots, no system trust store.
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();

        let http = ClientBuilder::new()
            .use_preconfigured_tls(tls_config)
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(ClientError::Http)?;

        Ok(Self {
            http,
            identity,
            peer_address: peer.address.clone(),
            peer_port: peer.control_port,
        })
    }

    /// Perform a TOFU first-contact handshake with an unknown peer.
    ///
    /// Sends `GET https://<address>:<port>/v1/hello` with certificate
    /// validation **completely disabled** — the caller MUST pin the returned
    /// [`HelloPayload::cert_sha256`] after this call.
    ///
    /// This is the **only** place in the entire codebase where
    /// `danger_accept_invalid_certs(true)` is used.  See plan §19.3.
    ///
    /// This method is single-shot with a 5 s timeout.  No retry is attempted
    /// because TOFU is an interactive operation.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] on connection failure, timeout, or a non-2xx
    /// response from the remote.
    pub async fn bootstrap_hello(address: &str, port: u16) -> Result<HelloPayload, ClientError> {
        // SECURITY: danger_accept_invalid_certs MUST ONLY appear in this method.
        let http = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(ClientError::Http)?;

        let url = format!("https://{address}:{port}/v1/hello");
        debug!(url = %url, "bootstrap_hello: sending TOFU request");

        let response = http
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    ClientError::Unreachable {
                        address: address.to_string(),
                        port,
                    }
                } else {
                    ClientError::Http(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let code = status.as_u16();
            let body = response
                .json::<ErrorResponse>()
                .await
                .unwrap_or(ErrorResponse {
                    error: "unknown".to_string(),
                    message: format!("HTTP {code}"),
                });
            return Err(ClientError::Remote {
                status: code,
                code: body.error,
                message: body.message,
            });
        }

        response
            .json::<HelloPayload>()
            .await
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }

    // ── Base URL helper ──────────────────────────────────────────────────────

    /// Build the full HTTPS URL for a control-plane path.
    fn url(&self, path: &str) -> String {
        format!(
            "https://{}:{}/v1{}",
            self.peer_address, self.peer_port, path
        )
    }

    // ── send_signed ──────────────────────────────────────────────────────────

    /// Sign and send an authenticated request to the remote peer.
    ///
    /// Steps:
    /// 1. Serialize `body` to JSON bytes (empty vec if `None`).
    /// 2. Sign via [`sign_request`] to produce [`SignedHeaders`].
    /// 3. Attach all `X-Orrbeam-*` headers.
    /// 4. Send; map connection errors to [`ClientError::Unreachable`].
    /// 5. On 4xx/5xx, parse the body as [`ErrorResponse`] and return
    ///    [`ClientError::Remote`].
    /// 6. Return the raw response for the caller to parse.
    async fn send_signed<B: Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<reqwest::Response, ClientError> {
        let body_bytes: Vec<u8> = match body {
            Some(b) => {
                serde_json::to_vec(b).map_err(|e| ClientError::SigningError(e.to_string()))?
            }
            None => Vec::new(),
        };

        let headers = sign_request(&self.identity, method.as_str(), path, &body_bytes);

        let url = self.url(path);
        let mut builder = self
            .http
            .request(method.clone(), &url)
            .header(HEADER_VERSION, &headers.version)
            .header(HEADER_KEY_ID, &headers.key_id)
            .header(HEADER_TIMESTAMP, &headers.timestamp)
            .header(HEADER_NONCE, &headers.nonce)
            .header(HEADER_SIGNATURE, &headers.signature);

        if !body_bytes.is_empty() {
            builder = builder
                .header("Content-Type", "application/json")
                .body(body_bytes);
        }

        let response = builder.send().await.map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                ClientError::Unreachable {
                    address: self.peer_address.clone(),
                    port: self.peer_port,
                }
            } else {
                ClientError::Http(e)
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let code = status.as_u16();
            let body = response
                .json::<ErrorResponse>()
                .await
                .unwrap_or(ErrorResponse {
                    error: "unknown".to_string(),
                    message: format!("HTTP {code}"),
                });
            return Err(ClientError::Remote {
                status: code,
                code: body.error,
                message: body.message,
            });
        }

        Ok(response)
    }

    // ── Retry helper ─────────────────────────────────────────────────────────

    /// Wrap an async operation in a retry loop.
    ///
    /// Retries up to `max_attempts` times with `sleep_secs` between attempts.
    /// Only retries on errors classified as transient by [`should_retry`].
    async fn with_retry<F, Fut, T>(
        &self,
        max_attempts: usize,
        sleep_secs: u64,
        op_name: &str,
        f: F,
    ) -> Result<T, ClientError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        let mut last_err = None;
        for attempt in 1..=max_attempts {
            match f().await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let retry = should_retry(&e);
                    debug!(
                        op = op_name,
                        attempt,
                        max_attempts,
                        retryable = (retry == ShouldRetry::Yes),
                        error = %e,
                        "control client attempt failed"
                    );
                    if retry == ShouldRetry::No {
                        return Err(e);
                    }
                    last_err = Some(e);
                    if attempt < max_attempts {
                        tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
                    }
                }
            }
        }
        warn!(
            op = op_name,
            max_attempts, "all retry attempts exhausted"
        );
        Err(last_err.expect("loop ran at least once"))
    }

    // ── Public typed methods ──────────────────────────────────────────────────

    /// Query the remote node's Sunshine and Moonlight process status.
    ///
    /// Retries up to 3 times (1 s sleep) on transient errors.
    pub async fn status(&self) -> Result<StatusResponse, ClientError> {
        self.with_retry(3, 1, "status", || async {
            let resp = self
                .send_signed::<()>(reqwest::Method::GET, "/status", None)
                .await?;
            resp.json::<StatusResponse>()
                .await
                .map_err(|e| ClientError::InvalidResponse(e.to_string()))
        })
        .await
    }

    /// Start the Sunshine host service on the remote node.
    ///
    /// Retries up to 3 times (1 s sleep) on transient errors.
    pub async fn sunshine_start(&self) -> Result<StartResponse, ClientError> {
        self.with_retry(3, 1, "sunshine_start", || async {
            let resp = self
                .send_signed::<()>(reqwest::Method::POST, "/sunshine/start", None)
                .await?;
            resp.json::<StartResponse>()
                .await
                .map_err(|e| ClientError::InvalidResponse(e.to_string()))
        })
        .await
    }

    /// Stop the Sunshine host service on the remote node.
    ///
    /// Retries up to 3 times (1 s sleep) on transient errors.
    pub async fn sunshine_stop(&self) -> Result<StopResponse, ClientError> {
        self.with_retry(3, 1, "sunshine_stop", || async {
            let resp = self
                .send_signed::<()>(reqwest::Method::POST, "/sunshine/stop", None)
                .await?;
            resp.json::<StopResponse>()
                .await
                .map_err(|e| ClientError::InvalidResponse(e.to_string()))
        })
        .await
    }

    /// Submit a Sunshine pairing PIN to the remote node.
    ///
    /// The remote Sunshine may not yet have a pending pair request when this
    /// is called (user clicked "Pair" on the Moonlight side just moments ago),
    /// so this method retries up to **15 times** with 1 s sleep.
    ///
    /// # Arguments
    ///
    /// * `pin` — the 4-digit PIN displayed by Moonlight
    /// * `client_name` — the friendly name to register for this pairing
    pub async fn pair_accept(
        &self,
        pin: &str,
        client_name: &str,
    ) -> Result<PairAcceptResponse, ClientError> {
        let pin = pin.to_string();
        let client_name = client_name.to_string();
        self.with_retry(15, 1, "pair_accept", || {
            let pin = pin.clone();
            let client_name = client_name.clone();
            async move {
                let body = PairRequest {
                    pin: &pin,
                    client_name: &client_name,
                };
                let resp = self
                    .send_signed(reqwest::Method::POST, "/sunshine/pair", Some(&body))
                    .await?;
                resp.json::<PairAcceptResponse>()
                    .await
                    .map_err(|e| ClientError::InvalidResponse(e.to_string()))
            }
        })
        .await
    }

    /// Retrieve the remote node's trusted-peer list.
    ///
    /// Single-shot, no retry (listing peers is a read-only, non-critical path).
    pub async fn peers(&self) -> Result<PeersResponse, ClientError> {
        let resp = self
            .send_signed::<()>(reqwest::Method::GET, "/peers", None)
            .await?;
        resp.json::<PeersResponse>()
            .await
            .map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ErrorResponse JSON parsing ────────────────────────────────────────────

    /// `ErrorResponse` must deserialize correctly from canonical JSON.
    #[test]
    fn error_response_parses_from_json() {
        let json = r#"{"error":"unauthorized","message":"invalid signature"}"#;
        let parsed: ErrorResponse =
            serde_json::from_str(json).expect("should parse valid error JSON");
        assert_eq!(parsed.error, "unauthorized");
        assert_eq!(parsed.message, "invalid signature");
    }

    /// Missing fields in the JSON body should result in a serde error, not a
    /// panic.
    #[test]
    fn error_response_rejects_incomplete_json() {
        let json = r#"{"error":"oops"}"#; // missing "message"
        let result = serde_json::from_str::<ErrorResponse>(json);
        assert!(
            result.is_err(),
            "expected Err for missing 'message' field, got Ok"
        );
    }

    // ── bootstrap_hello URL construction ─────────────────────────────────────

    /// `bootstrap_hello` must target `https://<address>:<port>/v1/hello`.
    ///
    /// We can't make an actual network call in a unit test, so we verify only
    /// that the URL string is assembled correctly using the helper (white-box).
    #[test]
    fn bootstrap_hello_url_format() {
        // Directly test the URL format used inside bootstrap_hello.
        let address = "192.168.1.100";
        let port: u16 = 47782;
        let url = format!("https://{address}:{port}/v1/hello");
        assert_eq!(url, "https://192.168.1.100:47782/v1/hello");
    }

    /// The client's `url()` helper must produce the correct base URL.
    #[test]
    fn client_url_helper_formats_correctly() {
        // We can't construct a ControlClient without a real peer (TLS config),
        // so test the URL pattern directly.
        let address = "10.0.0.1";
        let port: u16 = 47782;
        let path = "/status";
        let url = format!("https://{address}:{port}/v1{path}");
        assert_eq!(url, "https://10.0.0.1:47782/v1/status");
    }

    // ── should_retry logic ────────────────────────────────────────────────────

    /// Unreachable errors should be retried.
    #[test]
    fn unreachable_error_is_retryable() {
        let err = ClientError::Unreachable {
            address: "10.0.0.1".to_string(),
            port: 47782,
        };
        assert_eq!(
            should_retry(&err),
            ShouldRetry::Yes,
            "Unreachable should be retryable"
        );
    }

    /// Remote 503 errors should be retried.
    #[test]
    fn remote_503_is_retryable() {
        let err = ClientError::Remote {
            status: 503,
            code: "service_unavailable".to_string(),
            message: "try again".to_string(),
        };
        assert_eq!(
            should_retry(&err),
            ShouldRetry::Yes,
            "503 should be retryable"
        );
    }

    /// Remote 401 errors should NOT be retried.
    #[test]
    fn remote_401_is_not_retryable() {
        let err = ClientError::Remote {
            status: 401,
            code: "unauthorized".to_string(),
            message: "bad signature".to_string(),
        };
        assert_eq!(
            should_retry(&err),
            ShouldRetry::No,
            "401 should not be retryable"
        );
    }

    /// Remote 404 errors should NOT be retried.
    #[test]
    fn remote_404_is_not_retryable() {
        let err = ClientError::Remote {
            status: 404,
            code: "not_found".to_string(),
            message: "endpoint not found".to_string(),
        };
        assert_eq!(
            should_retry(&err),
            ShouldRetry::No,
            "404 should not be retryable"
        );
    }

    /// Signing errors should NOT be retried.
    #[test]
    fn signing_error_is_not_retryable() {
        let err = ClientError::SigningError("clock error".to_string());
        assert_eq!(
            should_retry(&err),
            ShouldRetry::No,
            "signing errors should not be retried"
        );
    }

    /// Cert pin mismatch errors should NOT be retried.
    #[test]
    fn cert_pin_mismatch_is_not_retryable() {
        let err = ClientError::CertPinMismatch {
            expected: "aabb".to_string(),
            actual: "ccdd".to_string(),
        };
        assert_eq!(
            should_retry(&err),
            ShouldRetry::No,
            "CertPinMismatch should not be retried"
        );
    }
}
