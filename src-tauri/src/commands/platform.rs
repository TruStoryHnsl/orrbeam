use crate::AppState;
use crate::error::AppError;
use orrbeam_platform::{GpuInfo, MonitorInfo, PlatformInfo};
use tauri::State;

#[tauri::command]
pub fn get_platform_info(state: State<'_, AppState>) -> PlatformInfo {
    state.platform.info()
}

/// GPU detection runs nvidia-smi/vainfo — must be async to avoid blocking the main thread.
#[tauri::command]
pub async fn get_gpu_info(state: State<'_, AppState>) -> Result<GpuInfo, AppError> {
    state.platform.gpu_info().map_err(AppError::from)
}

/// Monitor enumeration runs xrandr/wlr-randr — must be async to avoid blocking the main thread.
#[tauri::command]
pub async fn get_monitors(state: State<'_, AppState>) -> Result<Vec<MonitorInfo>, AppError> {
    state.platform.monitors().map_err(AppError::from)
}
