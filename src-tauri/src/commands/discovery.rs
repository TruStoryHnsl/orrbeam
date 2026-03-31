use crate::AppState;
use orrbeam_core::Node;
use tauri::State;

#[tauri::command]
pub async fn get_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, String> {
    let registry = state.registry.read().await;
    Ok(registry.all().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn get_node_count(state: State<'_, AppState>) -> Result<usize, String> {
    let registry = state.registry.read().await;
    Ok(registry.online_count())
}
