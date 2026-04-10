use crate::AppState;
use orrbeam_core::identity::PublicIdentity;
use orrbeam_core::Config;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    let mut current = state.config.write().await;
    *current = config;
    Ok(())
}

#[tauri::command]
pub async fn get_identity(state: State<'_, AppState>) -> Result<PublicIdentity, String> {
    Ok(state.identity.public_identity())
}

/// TLS and identity fingerprints returned by [`get_tls_fingerprint`].
///
/// Intended for display in the About tab so the user can verify their node's
/// identity out-of-band (e.g., by comparing fingerprints with a peer over a
/// trusted side channel).
#[derive(Debug, Serialize)]
pub struct TlsFingerprint {
    /// SHA-256 hex of the TLS certificate (64 lowercase hex chars).
    pub cert_sha256: String,
    /// Ed25519 fingerprint — first 16 hex chars of the node's public key.
    pub ed25519_fingerprint: String,
    /// The control-plane HTTPS port.
    pub control_port: u16,
}

/// Return the node's TLS certificate fingerprint and Ed25519 identity for
/// out-of-band peer verification.
///
/// The TLS identity is loaded from disk (or generated on first call).  The
/// Ed25519 fingerprint is derived from the node's in-memory signing key.
/// The control port is the well-known orrbeam control-plane port (47782).
#[tauri::command]
pub async fn get_tls_fingerprint(
    state: State<'_, AppState>,
) -> Result<TlsFingerprint, String> {
    let config = state.config.read().await;
    let tls = orrbeam_core::tls::TlsIdentity::load_or_create(&state.identity, &config.node_name)
        .map_err(|e| e.to_string())?;
    Ok(TlsFingerprint {
        cert_sha256: tls.cert_sha256_hex,
        ed25519_fingerprint: state.identity.fingerprint(),
        control_port: config.api_port,
    })
}
