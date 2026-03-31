use crate::AppState;
use orrbeam_core::identity::PublicIdentity;
use orrbeam_core::Config;
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
pub fn get_identity(state: State<'_, AppState>) -> PublicIdentity {
    state.identity.public_identity()
}
