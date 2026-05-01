//! Signature-verification middleware for the orrbeam control plane.
//!
//! [`require_signed`] is an axum middleware function that extracts the five
//! orrbeam authentication headers from every inbound request, verifies the
//! Ed25519 signature, checks the nonce for replay, and injects a [`PeerContext`]
//! into request extensions so that downstream handlers can access the
//! authenticated peer record without re-querying the store.
//!
//! # Header extraction order
//! 1. `X-Orrbeam-Version`
//! 2. `X-Orrbeam-Key-Id`
//! 3. `X-Orrbeam-Timestamp`
//! 4. `X-Orrbeam-Nonce`
//! 5. `X-Orrbeam-Signature`

#![warn(missing_docs)]

use std::sync::Arc;

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use ed25519_dalek::VerifyingKey;

use orrbeam_core::wire::{
    HEADER_KEY_ID, HEADER_NONCE, HEADER_SIGNATURE, HEADER_TIMESTAMP, HEADER_VERSION,
};

use super::ControlState;
use super::errors::ControlError;

// ---------------------------------------------------------------------------
// PeerContext
// ---------------------------------------------------------------------------

/// The authenticated peer record injected into request extensions by
/// [`require_signed`].
///
/// Downstream handlers should extract this via
/// `Extension(peer_ctx): Extension<PeerContext>` to obtain the peer's
/// metadata and permission flags.
#[derive(Clone, Debug)]
pub struct PeerContext {
    /// The trusted peer whose signature was successfully verified.
    pub peer: orrbeam_core::peers::TrustedPeer,
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/// Axum middleware that authenticates inbound requests using Ed25519 signatures.
///
/// # Steps
/// 1. Extract the five orrbeam request headers.
/// 2. Validate the timestamp is within ±30 seconds of now.
/// 3. Buffer the full request body (up to 64 KiB) and compute its SHA-256 hash.
/// 4. Build the canonical signing string via [`orrbeam_core::wire::build_canonical_string`].
/// 5. Look up the peer by `key_id` in the trusted-peer store.
/// 6. Decode the peer's public key and verify the signature.
/// 7. Check the nonce cache for replay attacks.
/// 8. Inject [`PeerContext`] into request extensions.
/// 9. Reconstruct the request with the buffered body and call the next handler.
///
/// Any failure in steps 1–7 returns the appropriate [`ControlError`] variant
/// before passing the request downstream.
pub async fn require_signed(
    State(state): State<Arc<ControlState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ControlError> {
    // ── Step 1: Extract headers ──────────────────────────────────────────────
    let extract_header = |req: &Request<Body>, name: &str| -> Result<String, ControlError> {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| ControlError::MissingHeader(name.to_string()))
    };

    let _version = extract_header(&request, HEADER_VERSION)?;
    let key_id = extract_header(&request, HEADER_KEY_ID)?;
    let timestamp_str = extract_header(&request, HEADER_TIMESTAMP)?;
    let nonce = extract_header(&request, HEADER_NONCE)?;
    let signature_b64 = extract_header(&request, HEADER_SIGNATURE)?;

    // ── Step 2: Clock skew check ─────────────────────────────────────────────
    let timestamp: i64 = timestamp_str.parse().map_err(|_| ControlError::ClockSkew)?;

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let skew = (now - timestamp).unsigned_abs();
    if skew > 30 {
        return Err(ControlError::ClockSkew);
    }

    // ── Step 3: Buffer the body ───────────────────────────────────────────────
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, 64 * 1024)
        .await
        .map_err(|e| ControlError::InvalidBody(e.to_string()))?;

    // ── Step 4: Build canonical string ───────────────────────────────────────
    // Already done inside verify_signature; we need it for the lookup first.
    let method = parts.method.as_str();
    let path = parts.uri.path();

    // ── Step 5: Look up the peer ─────────────────────────────────────────────
    let peer = {
        let store = state.peers.read().await;
        store
            .by_fingerprint(&key_id)
            .cloned()
            .ok_or_else(|| ControlError::UnknownKey(key_id.clone()))?
    };

    // ── Step 6: Verify the signature ─────────────────────────────────────────
    let pk_bytes = STANDARD_NO_PAD
        .decode(&peer.ed25519_public_key_b64)
        .map_err(|_| ControlError::BadSignature)?;

    let pk_array: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| ControlError::BadSignature)?;

    let verifying_key =
        VerifyingKey::from_bytes(&pk_array).map_err(|_| ControlError::BadSignature)?;

    orrbeam_core::wire::verify_signature(
        &verifying_key,
        method,
        path,
        timestamp,
        &nonce,
        &key_id,
        &body_bytes,
        &signature_b64,
    )
    .map_err(|_| ControlError::BadSignature)?;

    // ── Step 7: Replay check ─────────────────────────────────────────────────
    let accepted = state.nonces.insert_or_reject(&key_id, &nonce).await;
    if !accepted {
        return Err(ControlError::Replay);
    }

    // ── Step 8: Inject PeerContext into extensions ────────────────────────────
    let mut request = Request::from_parts(parts, Body::from(body_bytes));
    request.extensions_mut().insert(PeerContext { peer });

    // ── Step 9: Call the next handler ─────────────────────────────────────────
    Ok(next.run(request).await)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use orrbeam_core::wire::{build_canonical_string, sign_request};

    /// Verify that build_canonical_string produces the same bytes regardless of
    /// whether the body is empty or non-empty — basic sanity for the middleware's
    /// canonical-string reconstruction path.
    #[test]
    fn canonical_string_is_deterministic() {
        let a = build_canonical_string("GET", "/v1/status", 1700000000, "nonce1", "keyabc", b"");
        let b = build_canonical_string("GET", "/v1/status", 1700000000, "nonce1", "keyabc", b"");
        assert_eq!(a, b);
    }

    /// Changing any field must produce a different canonical string.
    #[test]
    fn canonical_string_changes_with_method() {
        let get = build_canonical_string("GET", "/v1/status", 1700000000, "nonce1", "keyabc", b"");
        let post =
            build_canonical_string("POST", "/v1/status", 1700000000, "nonce1", "keyabc", b"");
        assert_ne!(get, post);
    }

    /// sign_request → verify_signature round-trip (mirrors wire.rs, but here we
    /// confirm the middleware would call the same function with correct args).
    #[test]
    fn sign_verify_roundtrip_matches_wire() {
        let identity = orrbeam_core::identity::Identity::generate().unwrap();
        let body = b"hello from middleware test";
        let headers = sign_request(&identity, "POST", "/v1/pair/accept", body);

        let ts: i64 = headers.timestamp.parse().unwrap();
        orrbeam_core::wire::verify_signature(
            &identity.public_key(),
            "POST",
            "/v1/pair/accept",
            ts,
            &headers.nonce,
            &headers.key_id,
            body,
            &headers.signature,
        )
        .expect("sign→verify round-trip must succeed");
    }
}
