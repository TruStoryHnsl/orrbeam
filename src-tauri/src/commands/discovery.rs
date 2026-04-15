use crate::AppState;
use orrbeam_core::{DiscoverySource, Node, NodeState};
use std::net::IpAddr;
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

/// Add or update a node in the registry and persist to disk.
///
/// `address` must be a valid IPv4 or IPv6 address string.
/// The node is inserted with `NodeState::Offline` so it renders greyed-out
/// until discovery confirms it online.
#[tauri::command]
pub async fn add_node(
    state: State<'_, AppState>,
    name: String,
    address: String,
    port: u16,
    fingerprint: Option<String>,
) -> Result<(), String> {
    let addr: IpAddr = address
        .parse()
        .map_err(|e| format!("invalid address '{address}': {e}"))?;

    let node = Node {
        name: name.clone(),
        address: addr,
        port,
        state: NodeState::Offline,
        source: DiscoverySource::Static,
        fingerprint,
        sunshine_available: false,
        moonlight_available: false,
        os: None,
        encoder: None,
        cert_sha256: None,
        last_seen: None,
    };

    let mut registry = state.registry.write().await;
    registry.upsert(node);

    let path = orrbeam_core::NodeRegistry::default_path();
    registry
        .save(&path)
        .map_err(|e| format!("failed to save registry: {e}"))?;

    tracing::info!(node = %name, "node added to persistent registry");
    Ok(())
}

/// Remove a node from the registry and persist to disk.
#[tauri::command]
pub async fn remove_node(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut registry = state.registry.write().await;
    let removed = registry.remove(&name);

    if removed.is_none() {
        return Err(format!("node '{name}' not found"));
    }

    let path = orrbeam_core::NodeRegistry::default_path();
    registry
        .save(&path)
        .map_err(|e| format!("failed to save registry: {e}"))?;

    tracing::info!(node = %name, "node removed from persistent registry");
    Ok(())
}

/// List all nodes (online and offline) from the registry.
///
/// Equivalent to `get_nodes` but makes the intent explicit — includes offline
/// nodes that are known from previous sessions.
#[tauri::command]
pub async fn list_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, String> {
    let registry = state.registry.read().await;
    Ok(registry.all().into_iter().cloned().collect())
}
