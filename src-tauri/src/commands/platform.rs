use crate::AppState;
use orrbeam_platform::{GpuInfo, MonitorInfo, PlatformInfo};
use tauri::State;

#[tauri::command]
pub fn get_platform_info(state: State<'_, AppState>) -> PlatformInfo {
    state.platform.info()
}

/// GPU detection runs nvidia-smi/vainfo — must be async to avoid blocking the main thread.
#[tauri::command]
pub async fn get_gpu_info(state: State<'_, AppState>) -> Result<GpuInfo, String> {
    let platform = &state.platform;
    // Platform trait methods use std::process::Command (blocking I/O).
    // In Tauri v2, sync commands run on the main thread which also runs the webview.
    // We must not block it — so we use the async variant which runs on the tokio pool.
    platform.gpu_info().map_err(|e| e.to_string())
}

/// Monitor enumeration runs xrandr/wlr-randr — must be async to avoid blocking the main thread.
#[tauri::command]
pub async fn get_monitors(state: State<'_, AppState>) -> Result<Vec<MonitorInfo>, String> {
    state.platform.monitors().map_err(|e| e.to_string())
}
