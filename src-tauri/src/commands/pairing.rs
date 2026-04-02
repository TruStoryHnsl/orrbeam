use crate::AppState;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Result of initiating a pairing request.
#[derive(Debug, Serialize, Deserialize)]
pub struct PairInitResult {
    pub pin: String,
    pub target: String,
    pub started: bool,
}

/// Initiate pairing with a remote Sunshine host.
///
/// Generates a 4-digit PIN, launches `moonlight-qt pair <address> --pin <PIN>`,
/// and returns the PIN for the user to enter on the remote machine's Sunshine.
#[tauri::command]
pub async fn pair_initiate(
    state: State<'_, AppState>,
    address: String,
) -> Result<PairInitResult, String> {
    let pin = format!("{:04}", rand::rng().random_range(0..10000u32));
    let config = state.config.read().await;

    state
        .platform
        .pair_moonlight(&config, &address, &pin)
        .map_err(|e| e.to_string())?;

    tracing::info!("Pairing initiated with {address}, PIN: {pin}");

    Ok(PairInitResult {
        pin: pin.clone(),
        target: address,
        started: true,
    })
}

/// Accept an incoming pairing request by submitting a PIN to the local Sunshine API.
///
/// When a remote Moonlight tries to pair with our Sunshine, Sunshine shows a
/// pending request. This command submits the PIN to Sunshine's local API to
/// complete the handshake.
#[tauri::command]
pub async fn pair_accept(
    state: State<'_, AppState>,
    pin: String,
    client_name: Option<String>,
) -> Result<bool, String> {
    let config = state.config.read().await;
    let name = client_name.unwrap_or_else(|| "remote".to_string());

    orrbeam_core::sunshine_api::submit_pin_local(
        &config.sunshine_username,
        &config.sunshine_password,
        &pin,
        &name,
        15,
    )
    .await
    .map(|()| true)
    .map_err(|e| e.to_string())
}
