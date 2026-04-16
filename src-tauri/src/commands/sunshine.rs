use crate::error::AppError;
use crate::AppState;
use orrbeam_core::sunshine_conf::{self, SunshineSettings};
use orrbeam_platform::ServiceInfo;
use tauri::State;

#[tauri::command]
pub async fn get_sunshine_status(state: State<'_, AppState>) -> Result<ServiceInfo, AppError> {
    let config = state.config.read().await;
    state
        .platform
        .sunshine_status(&config)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn start_sunshine(state: State<'_, AppState>) -> Result<(), AppError> {
    let config = state.config.read().await;
    state
        .platform
        .start_sunshine(&config)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn stop_sunshine(state: State<'_, AppState>) -> Result<(), AppError> {
    state.platform.stop_sunshine().map_err(AppError::from)
}

#[tauri::command]
pub async fn get_sunshine_settings() -> Result<SunshineSettings, AppError> {
    sunshine_conf::get_settings().map_err(AppError::from)
}

#[tauri::command]
pub async fn set_sunshine_settings(settings: SunshineSettings) -> Result<(), AppError> {
    sunshine_conf::set_settings(&settings).map_err(AppError::from)
}

#[tauri::command]
pub async fn set_sunshine_monitor(monitor: String) -> Result<(), AppError> {
    if monitor.trim().is_empty() {
        return Err(AppError::InvalidInput("monitor name must not be empty".into()));
    }
    let mut updates = std::collections::HashMap::new();
    updates.insert("output_name".to_string(), monitor);
    sunshine_conf::write_conf(&updates).map_err(AppError::from)
}
