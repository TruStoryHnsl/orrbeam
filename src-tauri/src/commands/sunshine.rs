use crate::AppState;
use orrbeam_platform::ServiceInfo;
use tauri::State;

#[tauri::command]
pub async fn get_sunshine_status(state: State<'_, AppState>) -> Result<ServiceInfo, String> {
    let config = state.config.read().await;
    state
        .platform
        .sunshine_status(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_sunshine(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.read().await;
    state
        .platform
        .start_sunshine(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_sunshine(state: State<'_, AppState>) -> Result<(), String> {
    state.platform.stop_sunshine().map_err(|e| e.to_string())
}
