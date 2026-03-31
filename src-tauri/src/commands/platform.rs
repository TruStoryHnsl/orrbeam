use crate::AppState;
use orrbeam_platform::{GpuInfo, MonitorInfo, PlatformInfo};
use tauri::State;

#[tauri::command]
pub fn get_platform_info(state: State<'_, AppState>) -> PlatformInfo {
    state.platform.info()
}

#[tauri::command]
pub fn get_gpu_info(state: State<'_, AppState>) -> Result<GpuInfo, String> {
    state.platform.gpu_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_monitors(state: State<'_, AppState>) -> Result<Vec<MonitorInfo>, String> {
    state.platform.monitors().map_err(|e| e.to_string())
}
