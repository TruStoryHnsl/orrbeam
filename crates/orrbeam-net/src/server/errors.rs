//! Error types for the orrbeam control-plane HTTPS server.
//!
//! [`ControlError`] covers every failure mode a handler or middleware can
//! produce. Each variant maps to a specific HTTP status code and a JSON body of
//! the form `{"error": "<code>", "message": "<human-readable detail>"}`.

#![warn(missing_docs)]

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Wire response body
// ---------------------------------------------------------------------------

/// JSON body returned by the server for every error response.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

// ---------------------------------------------------------------------------
// ControlError
// ---------------------------------------------------------------------------

/// All errors that can be returned by control-plane handlers.
///
/// Implements [`axum::response::IntoResponse`] so handlers can use the
/// `?`-operator directly with `Result<_, ControlError>` return types.
#[derive(Debug)]
pub enum ControlError {
    /// A required orrbeam request header was absent.
    MissingHeader(String),
    /// Ed25519 signature verification failed.
    BadSignature,
    /// No trusted peer with the supplied key-id fingerprint was found.
    UnknownKey(String),
    /// Request timestamp is outside the +-30-second acceptance window.
    ClockSkew,
    /// The supplied nonce has already been seen (replay attack).
    Replay,
    /// The authenticated peer lacks the required permission.
    Forbidden(String),
    /// The request body could not be parsed or validated.
    InvalidBody(String),
    /// An unexpected internal error occurred.
    Internal(String),
    /// The local Sunshine HTTP API could not be reached.
    SunshineUnreachable,
    /// Sunshine rejected the pairing PIN after the maximum number of attempts.
    PinRejected,
    /// A dependent service is temporarily unavailable.
    ServiceUnavailable(String),
    /// This client has made too many requests.
    RateLimited,
    /// A mutual-trust (TOFU) request is already pending for this receiver.
    TofuPending,
    /// The mutual-trust request was not found or has expired.
    TofuExpired,
    /// The shared-control session is not active on this node.
    SharedControlUnavailable,
}

impl IntoResponse for ControlError {
    fn into_response(self) -> Response {
        let (status, code, message): (StatusCode, &str, String) = match &self {
            Self::MissingHeader(h) => (
                StatusCode::UNAUTHORIZED,
                "missing_header",
                format!("missing header: {h}"),
            ),
            Self::BadSignature => (
                StatusCode::UNAUTHORIZED,
                "bad_signature",
                "signature verification failed".into(),
            ),
            Self::UnknownKey(k) => (
                StatusCode::UNAUTHORIZED,
                "unknown_key",
                format!("no trusted peer with key {k}"),
            ),
            Self::ClockSkew => (
                StatusCode::UNAUTHORIZED,
                "clock_skew",
                "timestamp outside +-30s window".into(),
            ),
            Self::Replay => (
                StatusCode::UNAUTHORIZED,
                "replay",
                "nonce already used".into(),
            ),
            Self::Forbidden(m) => (StatusCode::FORBIDDEN, "forbidden", m.clone()),
            Self::InvalidBody(m) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_body", m.clone()),
            Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m.clone()),
            Self::SunshineUnreachable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "sunshine_unreachable",
                "local Sunshine API not reachable".into(),
            ),
            Self::PinRejected => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "pin_rejected",
                "Sunshine rejected the PIN after max attempts".into(),
            ),
            Self::ServiceUnavailable(m) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                m.clone(),
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests".into(),
            ),
            Self::TofuPending => (
                StatusCode::CONFLICT,
                "tofu_pending",
                "a mutual trust request is already pending".into(),
            ),
            Self::TofuExpired => (
                StatusCode::GONE,
                "tofu_expired",
                "mutual trust request has expired".into(),
            ),
            Self::SharedControlUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "shared_control_unavailable",
                "shared-control session is not active".into(),
            ),
        };

        (
            status,
            Json(ErrorBody {
                error: code.to_string(),
                message,
            }),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    /// Decode the JSON body of an error response and return (status, error-code, message).
    async fn decode(err: ControlError) -> (StatusCode, String, String) {
        let resp = err.into_response();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("body bytes");
        let v: serde_json::Value =
            serde_json::from_slice(&body).expect("error body must be valid JSON");
        (
            status,
            v["error"].as_str().unwrap_or("").to_string(),
            v["message"].as_str().unwrap_or("").to_string(),
        )
    }

    #[tokio::test]
    async fn missing_header_is_401_with_code() {
        let (s, code, msg) = decode(ControlError::MissingHeader("X-Foo".into())).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "missing_header");
        assert!(msg.contains("X-Foo"));
    }

    #[tokio::test]
    async fn bad_signature_is_401() {
        let (s, code, _) = decode(ControlError::BadSignature).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "bad_signature");
    }

    #[tokio::test]
    async fn unknown_key_is_401() {
        let (s, code, msg) = decode(ControlError::UnknownKey("aabbccdd".into())).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "unknown_key");
        assert!(msg.contains("aabbccdd"));
    }

    #[tokio::test]
    async fn clock_skew_is_401() {
        let (s, code, _) = decode(ControlError::ClockSkew).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "clock_skew");
    }

    #[tokio::test]
    async fn replay_is_401() {
        let (s, code, _) = decode(ControlError::Replay).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "replay");
    }

    #[tokio::test]
    async fn forbidden_is_403() {
        let (s, code, msg) = decode(ControlError::Forbidden("no permission".into())).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        assert_eq!(code, "forbidden");
        assert!(msg.contains("no permission"));
    }

    #[tokio::test]
    async fn invalid_body_is_422() {
        let (s, code, _) = decode(ControlError::InvalidBody("bad json".into())).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(code, "invalid_body");
    }

    #[tokio::test]
    async fn internal_is_500() {
        let (s, code, _) = decode(ControlError::Internal("oops".into())).await;
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "internal");
    }

    #[tokio::test]
    async fn sunshine_unreachable_is_503() {
        let (s, code, _) = decode(ControlError::SunshineUnreachable).await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "sunshine_unreachable");
    }

    #[tokio::test]
    async fn pin_rejected_is_500() {
        let (s, code, _) = decode(ControlError::PinRejected).await;
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "pin_rejected");
    }

    #[tokio::test]
    async fn service_unavailable_is_503() {
        let (s, code, _) = decode(ControlError::ServiceUnavailable("db down".into())).await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "service_unavailable");
    }

    #[tokio::test]
    async fn rate_limited_is_429() {
        let (s, code, _) = decode(ControlError::RateLimited).await;
        assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(code, "rate_limited");
    }

    #[tokio::test]
    async fn tofu_pending_is_409() {
        let (s, code, _) = decode(ControlError::TofuPending).await;
        assert_eq!(s, StatusCode::CONFLICT);
        assert_eq!(code, "tofu_pending");
    }

    #[tokio::test]
    async fn tofu_expired_is_410() {
        let (s, code, _) = decode(ControlError::TofuExpired).await;
        assert_eq!(s, StatusCode::GONE);
        assert_eq!(code, "tofu_expired");
    }

    /// Every error variant must produce valid JSON with `error` and `message` keys.
    #[tokio::test]
    async fn all_variants_produce_json_with_required_keys() {
        let variants: Vec<ControlError> = vec![
            ControlError::MissingHeader("H".into()),
            ControlError::BadSignature,
            ControlError::UnknownKey("k".into()),
            ControlError::ClockSkew,
            ControlError::Replay,
            ControlError::Forbidden("f".into()),
            ControlError::InvalidBody("b".into()),
            ControlError::Internal("i".into()),
            ControlError::SunshineUnreachable,
            ControlError::PinRejected,
            ControlError::ServiceUnavailable("s".into()),
            ControlError::RateLimited,
            ControlError::TofuPending,
            ControlError::TofuExpired,
            ControlError::SharedControlUnavailable,
        ];

        for err in variants {
            let resp = err.into_response();
            let body = axum::body::to_bytes(resp.into_body(), 4096)
                .await
                .expect("body bytes");
            let v: serde_json::Value = serde_json::from_slice(&body).expect("must be valid JSON");
            assert!(v["error"].is_string(), "missing 'error' key");
            assert!(v["message"].is_string(), "missing 'message' key");
        }
    }
}
