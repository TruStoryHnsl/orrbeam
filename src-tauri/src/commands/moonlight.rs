use crate::AppState;
use orrbeam_platform::ServiceInfo;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct ConnectParams {
    pub address: String,
    pub app: Option<String>,
    pub windowed: Option<bool>,
    pub resolution: Option<String>,
}

#[tauri::command]
pub async fn get_moonlight_status(state: State<'_, AppState>) -> Result<ServiceInfo, String> {
    let config = state.config.read().await;
    state
        .platform
        .moonlight_status(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_moonlight(
    state: State<'_, AppState>,
    params: ConnectParams,
) -> Result<(), String> {
    let config = state.config.read().await;
    state
        .platform
        .start_moonlight(
            &config,
            &params.address,
            params.app.as_deref().unwrap_or("Desktop"),
            params.windowed.unwrap_or(false),
            params.resolution.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_moonlight(state: State<'_, AppState>) -> Result<(), String> {
    state.platform.stop_moonlight().map_err(|e| e.to_string())
}
