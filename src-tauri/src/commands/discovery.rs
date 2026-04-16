use crate::AppState;
use orrbeam_core::{Node, NodeState};
use serde::Deserialize;
use std::net::IpAddr;
use tauri::State;
use tracing::{info, warn};

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

/// Input for manually adding a node to the registry.
#[derive(Debug, Deserialize)]
pub struct AddNodeInput {
    pub name: String,
    pub address: String,
    pub port: u16,
}

/// Add a node to the registry manually and persist the registry to disk.
///
/// The node is added with `state = offline` (unknown reachability until
/// discovery confirms it). Returns an error if the address is not a valid IP.
#[tauri::command]
pub async fn add_node(state: State<'_, AppState>, node: AddNodeInput) -> Result<(), String> {
    let address: IpAddr = node
        .address
        .parse()
        .map_err(|_| format!("invalid IP address: {}", node.address))?;

    let new_node = Node {
        name: node.name.clone(),
        address,
        port: node.port,
        state: NodeState::Offline,
        source: orrbeam_core::node::DiscoverySource::Static,
        fingerprint: None,
        sunshine_available: false,
        moonlight_available: false,
        os: None,
        encoder: None,
        cert_sha256: None,
        last_seen: None,
    };

    let mut registry = state.registry.write().await;
    registry.add_manual(new_node);
    info!(name = %node.name, "node added manually");

    if let Err(e) = registry.save() {
        warn!(error = %e, "failed to persist node registry");
    }

    Ok(())
}

/// Remove a node from the registry by name and persist the change.
#[tauri::command]
pub async fn remove_node(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut registry = state.registry.write().await;
    let removed = registry.remove(&name);
    if removed.is_none() {
        return Err(format!("node '{}' not found", name));
    }
    info!(%name, "node removed");

    if let Err(e) = registry.save() {
        warn!(error = %e, "failed to persist node registry");
    }

    Ok(())
}

/// List all nodes in the registry (online and offline).
#[tauri::command]
pub async fn list_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, String> {
    let registry = state.registry.read().await;
    Ok(registry.all().into_iter().cloned().collect())
}
