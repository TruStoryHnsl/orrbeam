//! Structured application error type for Tauri IPC commands.
//!
//! All `#[tauri::command]` functions return `Result<T, AppError>` instead of
//! `Result<T, String>`. `AppError` serializes to a JSON object with `code` and
//! `message` fields so the frontend can distinguish error categories without
//! parsing human-readable strings.
//!
//! # Frontend usage
//!
//! ```typescript
//! interface AppError {
//!   code: string;
//!   message: string;
//! }
//! ```

use serde::Serialize;
use thiserror::Error;

/// All errors that can be returned by Tauri IPC commands.
#[derive(Debug, Error)]
pub enum AppError {
    /// The operation failed because the provided input is invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A node or peer with the requested identifier was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The platform abstraction (Sunshine/Moonlight process management) failed.
    #[error("platform error: {0}")]
    Platform(String),

    /// Configuration loading or saving failed.
    #[error("config error: {0}")]
    Config(String),

    /// The node identity or TLS certificate operation failed.
    #[error("identity error: {0}")]
    Identity(String),

    /// A network or control-plane operation failed.
    #[error("network error: {0}")]
    Network(String),

    /// An unexpected internal error that does not fit another category.
    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON wire representation of [`AppError`] sent to the frontend.
///
/// Serializes as `{ "code": "...", "message": "..." }`.
#[derive(Serialize)]
struct AppErrorBody {
    code: &'static str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (code, message): (&'static str, String) = match self {
            Self::InvalidInput(m) => ("invalid_input", m.clone()),
            Self::NotFound(m) => ("not_found", m.clone()),
            Self::Platform(m) => ("platform_error", m.clone()),
            Self::Config(m) => ("config_error", m.clone()),
            Self::Identity(m) => ("identity_error", m.clone()),
            Self::Network(m) => ("network_error", m.clone()),
            Self::Internal(m) => ("internal_error", m.clone()),
        };
        AppErrorBody { code, message }.serialize(serializer)
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl From<orrbeam_platform::PlatformError> for AppError {
    fn from(e: orrbeam_platform::PlatformError) -> Self {
        AppError::Platform(e.to_string())
    }
}

impl From<orrbeam_core::config::ConfigError> for AppError {
    fn from(e: orrbeam_core::config::ConfigError) -> Self {
        AppError::Config(e.to_string())
    }
}

impl From<orrbeam_core::identity::IdentityError> for AppError {
    fn from(e: orrbeam_core::identity::IdentityError) -> Self {
        AppError::Identity(e.to_string())
    }
}

impl From<orrbeam_core::tls::TlsError> for AppError {
    fn from(e: orrbeam_core::tls::TlsError) -> Self {
        AppError::Identity(e.to_string())
    }
}

impl From<orrbeam_core::sunshine_conf::SunshineConfError> for AppError {
    fn from(e: orrbeam_core::sunshine_conf::SunshineConfError) -> Self {
        AppError::Config(e.to_string())
    }
}

impl From<orrbeam_net::client::ClientError> for AppError {
    fn from(e: orrbeam_net::client::ClientError) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<orrbeam_core::peers::PeersError> for AppError {
    fn from(e: orrbeam_core::peers::PeersError) -> Self {
        AppError::Internal(e.to_string())
    }
}
