//! Wire protocol types, signing, and verification for the orrbeam control plane.
//!
//! Every authenticated HTTPS request between two orrbeam nodes uses Ed25519
//! signatures over a canonical payload string. This module is the **single source
//! of truth** for how that canonical string is constructed — both the signing
//! client and the verifying server use these exact functions.
//!
//! # Protocol summary
//!
//! The signed bytes are the following newline-delimited ASCII string:
//!
//! ```text
//! orrbeam/1\n
//! <UPPERCASE METHOD>\n
//! <REQUEST PATH, no query string>\n
//! <Unix timestamp (seconds) as decimal string>\n
//! <32 hex-char nonce>\n
//! <Key-Id (fingerprint) as string>\n
//! <hex lowercase sha256 of exact request body bytes>\n
//! ```
//!
//! An empty body produces the well-known SHA-256 hash
//! `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

#![warn(missing_docs)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use ed25519_dalek::{Signature, Signer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::identity::Identity;
use ed25519_dalek::VerifyingKey;

// ── Header name constants ────────────────────────────────────────────────────

/// HTTP header name carrying the protocol version string.
pub const HEADER_VERSION: &str = "X-Orrbeam-Version";

/// HTTP header name carrying the signing key fingerprint.
pub const HEADER_KEY_ID: &str = "X-Orrbeam-Key-Id";

/// HTTP header name carrying the Unix timestamp (seconds) used during signing.
pub const HEADER_TIMESTAMP: &str = "X-Orrbeam-Timestamp";

/// HTTP header name carrying the random nonce used during signing.
pub const HEADER_NONCE: &str = "X-Orrbeam-Nonce";

/// HTTP header name carrying the base64-encoded Ed25519 signature.
pub const HEADER_SIGNATURE: &str = "X-Orrbeam-Signature";

/// Canonical protocol version string included in every signed payload.
pub const PROTOCOL_VERSION: &str = "orrbeam/1";

// ── Error type ───────────────────────────────────────────────────────────────

/// Errors that can occur during wire-protocol signing or verification.
#[derive(Error, Debug)]
pub enum WireError {
    /// The base64-encoded signature field could not be decoded.
    #[error("invalid base64 in signature")]
    InvalidSignature,

    /// The Ed25519 signature did not match the reconstructed canonical payload.
    #[error("signature verification failed")]
    BadSignature,

    /// A required request header was absent.
    #[error("missing required header: {0}")]
    MissingHeader(String),

    /// The timestamp header value could not be parsed as an integer.
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
}

// ── SignedHeaders ─────────────────────────────────────────────────────────────

/// The set of HTTP headers that authenticate an orrbeam control-plane request.
///
/// Attach these headers (using the [`HEADER_*`] constants as names) to every
/// outgoing request that requires authentication.
#[derive(Debug, Clone)]
pub struct SignedHeaders {
    /// Protocol version; always `"orrbeam/1"`.
    pub version: String,
    /// Signing key fingerprint (first 16 hex chars of the Ed25519 public key).
    pub key_id: String,
    /// Unix timestamp (seconds) at signing time, encoded as a decimal string.
    pub timestamp: String,
    /// 32 hex-char random nonce generated at signing time.
    pub nonce: String,
    /// Base64 (standard, no padding) encoding of the 64-byte Ed25519 signature.
    pub signature: String,
}

// ── HelloPayload ─────────────────────────────────────────────────────────────

/// Public identity information exchanged during TOFU and mutual trust setup.
///
/// Sent in the body of a `/hello` request so that a peer can learn this node's
/// cryptographic identity, capabilities, and listening port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloPayload {
    /// Human-readable node name (hostname or configured alias).
    pub node_name: String,
    /// Hex fingerprint of the Ed25519 public key (first 16 hex chars).
    pub ed25519_fingerprint: String,
    /// Full Ed25519 public key encoded as base64 standard no-pad.
    pub ed25519_public_key_b64: String,
    /// SHA-256 fingerprint (hex) of the node's TLS certificate.
    pub cert_sha256: String,
    /// TCP port on which this node's control-plane HTTPS server listens.
    pub control_port: u16,
    /// Whether Sunshine (remote-desktop host) is available and running.
    pub sunshine_available: bool,
    /// Whether Moonlight (remote-desktop client) is available and running.
    pub moonlight_available: bool,
    /// Operating system identifier (e.g. `"linux"`, `"macos"`, `"windows"`).
    pub os: String,
    /// Protocol version string; always `"orrbeam/1"`.
    pub version: String,
}

// ── Core functions ────────────────────────────────────────────────────────────

/// Generate a cryptographically random 32-character hex nonce.
///
/// The nonce is 128 bits of entropy from the OS CSPRNG, hex-encoded to 32
/// lowercase ASCII characters. It is included in every signed payload to
/// prevent replay attacks.
pub fn generate_nonce() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(bytes)
}

/// Compute the lowercase hex SHA-256 digest of `body`.
///
/// An empty `body` slice returns the well-known empty-body hash
/// `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
///
/// This helper is also useful for logging or debugging body integrity.
pub fn body_hash(body: &[u8]) -> String {
    hex::encode(Sha256::digest(body))
}

/// Build the canonical byte string that is signed and verified.
///
/// The format is exactly seven newline-terminated lines:
///
/// ```text
/// orrbeam/1\n
/// <UPPERCASE METHOD>\n
/// <path>\n
/// <timestamp>\n
/// <nonce>\n
/// <key_id>\n
/// <hex sha256 of body>\n
/// ```
///
/// Both the signer ([`sign_request`]) and the verifier ([`verify_signature`])
/// call this function with the same arguments; any deviation will produce a
/// mismatch. **Do not change this format without a protocol version bump.**
pub fn build_canonical_string(
    method: &str,
    path: &str,
    timestamp: i64,
    nonce: &str,
    key_id: &str,
    body: &[u8],
) -> Vec<u8> {
    let hash = body_hash(body);
    let s = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        PROTOCOL_VERSION,
        method.to_uppercase(),
        path,
        timestamp,
        nonce,
        key_id,
        hash,
    );
    s.into_bytes()
}

/// Sign an outgoing request and return the headers to attach.
///
/// This function:
/// 1. Derives the current Unix timestamp.
/// 2. Generates a fresh nonce.
/// 3. Builds the canonical signing payload via [`build_canonical_string`].
/// 4. Signs it with `identity`'s Ed25519 signing key.
/// 5. Base64-encodes the signature (standard, no padding).
///
/// Attach the returned [`SignedHeaders`] to the request using the
/// [`HEADER_*`] constants as header names.
pub fn sign_request(identity: &Identity, method: &str, path: &str, body: &[u8]) -> SignedHeaders {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let nonce = generate_nonce();
    let key_id = identity.fingerprint();

    let canonical = build_canonical_string(method, path, timestamp, &nonce, &key_id, body);
    let signature: ed25519_dalek::Signature = identity.signing_key().sign(&canonical);
    let signature_b64 = STANDARD_NO_PAD.encode(signature.to_bytes());

    SignedHeaders {
        version: PROTOCOL_VERSION.to_string(),
        key_id,
        timestamp: timestamp.to_string(),
        nonce,
        signature: signature_b64,
    }
}

/// Verify a signed request received from a peer.
///
/// The caller is responsible for extracting all header values and passing them
/// here. On success the function returns `Ok(())`; on any failure it returns
/// the appropriate [`WireError`] variant.
///
/// **Uses `verify_strict`** to reject signature variants that are formally
/// valid under the Ed25519 spec but exploitable via signature malleability.
#[allow(clippy::too_many_arguments)]
pub fn verify_signature(
    public_key: &VerifyingKey,
    method: &str,
    path: &str,
    timestamp: i64,
    nonce: &str,
    key_id: &str,
    body: &[u8],
    signature_b64: &str,
) -> Result<(), WireError> {
    // 1. Decode the base64 signature.
    let sig_bytes = STANDARD_NO_PAD
        .decode(signature_b64)
        .map_err(|_| WireError::InvalidSignature)?;

    // 2. Parse into a typed Ed25519 signature (must be exactly 64 bytes).
    let sig_array: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| WireError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_array);

    // 3. Reconstruct the canonical payload the signer would have produced.
    let canonical = build_canonical_string(method, path, timestamp, nonce, key_id, body);

    // 4. Verify — strict mode rejects malleable variants.
    public_key
        .verify_strict(&canonical, &signature)
        .map_err(|_| WireError::BadSignature)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    fn make_identity() -> Identity {
        Identity::generate().expect("generate identity")
    }

    // ── generate_nonce ────────────────────────────────────────────────────────

    #[test]
    fn nonce_is_32_hex_chars() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32, "nonce must be 32 chars");
        assert!(
            nonce.chars().all(|c| c.is_ascii_hexdigit()),
            "nonce must be lowercase hex: {nonce}"
        );
    }

    #[test]
    fn nonces_are_unique() {
        let a = generate_nonce();
        let b = generate_nonce();
        assert_ne!(a, b, "two nonces must differ");
    }

    // ── body_hash / empty body ────────────────────────────────────────────────

    #[test]
    fn empty_body_hash_is_correct() {
        const EMPTY_SHA256: &str =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(body_hash(b""), EMPTY_SHA256);
    }

    // ── build_canonical_string ────────────────────────────────────────────────

    #[test]
    fn canonical_string_empty_body_contains_empty_hash() {
        const EMPTY_SHA256: &str =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let canonical =
            build_canonical_string("GET", "/hello", 1700000000, "aabbcc", "abcd1234", b"");
        let s = String::from_utf8(canonical).unwrap();
        assert!(
            s.contains(EMPTY_SHA256),
            "canonical string must contain empty-body hash, got:\n{s}"
        );
    }

    #[test]
    fn canonical_string_format() {
        let canonical = build_canonical_string(
            "post",
            "/api/v1/foo",
            1700000001,
            "nonce123",
            "key456",
            b"hello",
        );
        let s = String::from_utf8(canonical).unwrap();
        let lines: Vec<&str> = s.split('\n').collect();
        // 7 non-empty lines + trailing empty after final \n
        assert_eq!(lines.len(), 8, "expected 7 lines + trailing empty: {s:?}");
        assert_eq!(lines[0], "orrbeam/1");
        assert_eq!(lines[1], "POST", "method must be uppercased");
        assert_eq!(lines[2], "/api/v1/foo");
        assert_eq!(lines[3], "1700000001");
        assert_eq!(lines[4], "nonce123");
        assert_eq!(lines[5], "key456");
        // line 6 is the body sha256 — just verify it's 64 hex chars
        assert_eq!(lines[6].len(), 64);
        assert_eq!(lines[7], "", "must end with trailing newline");
    }

    // ── sign_request ─────────────────────────────────────────────────────────

    #[test]
    fn signed_headers_have_correct_format() {
        let identity = make_identity();
        let headers = sign_request(&identity, "POST", "/v1/hello", b"{}");

        assert_eq!(headers.version, PROTOCOL_VERSION);
        assert_eq!(headers.key_id, identity.fingerprint());
        assert_eq!(headers.nonce.len(), 32);
        assert!(headers.nonce.chars().all(|c| c.is_ascii_hexdigit()));
        // timestamp is a valid integer
        headers
            .timestamp
            .parse::<i64>()
            .expect("timestamp must be an integer");
        // signature is valid base64 that decodes to 64 bytes
        let sig_bytes = STANDARD_NO_PAD
            .decode(&headers.signature)
            .expect("base64 decode");
        assert_eq!(sig_bytes.len(), 64, "Ed25519 signature must be 64 bytes");
    }

    // ── verify_signature — round-trip ─────────────────────────────────────────

    #[test]
    fn sign_then_verify_ok() {
        let identity = make_identity();
        let body = b"hello world";
        let headers = sign_request(&identity, "POST", "/api/test", body);
        let ts: i64 = headers.timestamp.parse().unwrap();

        verify_signature(
            &identity.public_key(),
            "POST",
            "/api/test",
            ts,
            &headers.nonce,
            &headers.key_id,
            body,
            &headers.signature,
        )
        .expect("verification must succeed with matching key");
    }

    #[test]
    fn verify_wrong_key_returns_bad_signature() {
        let identity_a = make_identity();
        let identity_b = make_identity();
        let body = b"payload";
        let headers = sign_request(&identity_a, "GET", "/nodes", body);
        let ts: i64 = headers.timestamp.parse().unwrap();

        let result = verify_signature(
            &identity_b.public_key(), // wrong key
            "GET",
            "/nodes",
            ts,
            &headers.nonce,
            &headers.key_id,
            body,
            &headers.signature,
        );
        assert!(
            matches!(result, Err(WireError::BadSignature)),
            "expected BadSignature, got {result:?}"
        );
    }

    #[test]
    fn verify_tampered_body_returns_bad_signature() {
        let identity = make_identity();
        let body = b"original body";
        let headers = sign_request(&identity, "POST", "/action", body);
        let ts: i64 = headers.timestamp.parse().unwrap();

        let tampered_body = b"tampered body";
        let result = verify_signature(
            &identity.public_key(),
            "POST",
            "/action",
            ts,
            &headers.nonce,
            &headers.key_id,
            tampered_body, // different body
            &headers.signature,
        );
        assert!(
            matches!(result, Err(WireError::BadSignature)),
            "expected BadSignature for tampered body, got {result:?}"
        );
    }

    #[test]
    fn verify_tampered_signature_returns_error() {
        let identity = make_identity();
        let body = b"payload";
        let headers = sign_request(&identity, "DELETE", "/peer/abc", body);
        let ts: i64 = headers.timestamp.parse().unwrap();

        // Corrupt the signature: replace last few chars with 'AAAA'
        let mut bad_sig = headers.signature.clone();
        let len = bad_sig.len();
        bad_sig.replace_range((len - 4).., "AAAA");

        let result = verify_signature(
            &identity.public_key(),
            "DELETE",
            "/peer/abc",
            ts,
            &headers.nonce,
            &headers.key_id,
            body,
            &bad_sig,
        );
        assert!(
            matches!(
                result,
                Err(WireError::InvalidSignature | WireError::BadSignature)
            ),
            "expected InvalidSignature or BadSignature for corrupted sig, got {result:?}"
        );
    }

    #[test]
    fn verify_garbage_base64_returns_invalid_signature() {
        let identity = make_identity();
        let body = b"";
        let headers = sign_request(&identity, "GET", "/ping", body);
        let ts: i64 = headers.timestamp.parse().unwrap();

        let result = verify_signature(
            &identity.public_key(),
            "GET",
            "/ping",
            ts,
            &headers.nonce,
            &headers.key_id,
            body,
            "this is not base64!!!",
        );
        assert!(
            matches!(result, Err(WireError::InvalidSignature)),
            "expected InvalidSignature for garbage base64, got {result:?}"
        );
    }
}
