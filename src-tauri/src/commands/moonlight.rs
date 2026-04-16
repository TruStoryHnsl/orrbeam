use crate::error::AppError;
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
pub async fn get_moonlight_status(state: State<'_, AppState>) -> Result<ServiceInfo, AppError> {
    let config = state.config.read().await;
    state
        .platform
        .moonlight_status(&config)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn start_moonlight(
    state: State<'_, AppState>,
    params: ConnectParams,
) -> Result<(), AppError> {
    // Input validation
    if params.address.trim().is_empty() {
        return Err(AppError::InvalidInput("address must not be empty".into()));
    }
    if let Some(ref res) = params.resolution {
        // Resolution must be in WxH format if provided
        if !res.is_empty() && !res.contains('x') {
            return Err(AppError::InvalidInput(
                "resolution must be in WxH format (e.g. '1920x1080')".into(),
            ));
        }
    }

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
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn stop_moonlight(state: State<'_, AppState>) -> Result<(), AppError> {
    state.platform.stop_moonlight().map_err(AppError::from)
}
