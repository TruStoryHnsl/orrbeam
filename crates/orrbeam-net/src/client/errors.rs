//! Error types for the orrbeam control-plane HTTP client.
//!
//! All fallible operations in [`super::ControlClient`] return a [`ClientError`].
//! The variants cover the full failure spectrum: network-level unreachability,
//! TLS cert-pin mismatches, remote application errors, and local signing
//! failures.

#![warn(missing_docs)]

use thiserror::Error;

/// Errors that can occur while sending a signed request to a remote peer's
/// control plane, or while processing the response.
#[derive(Error, Debug)]
pub enum ClientError {
    /// The underlying HTTP transport layer returned an error (connection
    /// refused, timeout, etc.).
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The remote peer's control plane returned a 4xx or 5xx status code.
    ///
    /// The `code` field is the machine-readable error tag from the response
    /// body; `message` is the human-readable explanation.
    #[error("control plane error {status}: {code} — {message}")]
    Remote {
        /// HTTP status code returned by the remote.
        status: u16,
        /// Machine-readable error code from the response body.
        code: String,
        /// Human-readable error message from the response body.
        message: String,
    },

    /// The TLS certificate presented by the remote peer did not match the
    /// SHA-256 fingerprint stored for that peer in the trusted-peer store.
    ///
    /// This indicates either a certificate rotation (resolve by re-running
    /// bootstrap TOFU) or a potential MITM.
    #[error("TLS cert pin mismatch: expected {expected}, got {actual}")]
    CertPinMismatch {
        /// The fingerprint that was expected (from the stored peer record).
        expected: String,
        /// The fingerprint of the certificate that was actually presented.
        actual: String,
    },

    /// The response body could not be parsed as the expected type, or another
    /// structural invariant was violated.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// The remote peer's control plane was unreachable at the given address and
    /// port (TCP connection refused, host unreachable, DNS failure, etc.).
    #[error("peer unreachable at {address}:{port}")]
    Unreachable {
        /// IP address or hostname of the peer.
        address: String,
        /// TCP port that was attempted.
        port: u16,
    },

    /// Building the signed request headers failed (e.g. clock is unavailable,
    /// or the identity key is somehow invalid).
    #[error("request signing failed: {0}")]
    SigningError(String),
}
