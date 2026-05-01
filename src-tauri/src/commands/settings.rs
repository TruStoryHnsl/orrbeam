use crate::AppState;
use crate::error::AppError;
use orrbeam_core::Config;
use orrbeam_core::identity::PublicIdentity;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, AppError> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), AppError> {
    config.save().map_err(AppError::from)?;
    let mut current = state.config.write().await;
    *current = config;
    Ok(())
}

#[tauri::command]
pub async fn get_identity(state: State<'_, AppState>) -> Result<PublicIdentity, AppError> {
    Ok(state.identity.public_identity())
}

/// TLS and identity fingerprints returned by [`get_tls_fingerprint`].
#[derive(Debug, Serialize)]
pub struct TlsFingerprint {
    /// SHA-256 hex of the TLS certificate (64 lowercase hex chars).
    pub cert_sha256: String,
    /// Ed25519 fingerprint — first 16 hex chars of the node's public key.
    pub ed25519_fingerprint: String,
    /// The control-plane HTTPS port.
    pub control_port: u16,
}

#[tauri::command]
pub async fn get_tls_fingerprint(state: State<'_, AppState>) -> Result<TlsFingerprint, AppError> {
    let config = state.config.read().await;
    let tls = orrbeam_core::tls::TlsIdentity::load_or_create(&state.identity, &config.node_name)
        .map_err(AppError::from)?;
    Ok(TlsFingerprint {
        cert_sha256: tls.cert_sha256_hex,
        ed25519_fingerprint: state.identity.fingerprint(),
        control_port: config.api_port,
    })
}
