use crate::AppState;
use crate::error::AppError;
use orrbeam_core::{DiscoverySource, Node, NodeState};
use serde::Deserialize;
use std::net::IpAddr;
use tauri::State;
use tracing::{info, warn};

#[tauri::command]
pub async fn get_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, AppError> {
    let registry = state.registry.read().await;
    Ok(registry.all().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn get_node_count(state: State<'_, AppState>) -> Result<usize, AppError> {
    let registry = state.registry.read().await;
    Ok(registry.online_count())
}

/// Input for manually adding a node to the registry.
#[derive(Debug, Deserialize)]
pub struct AddNodeInput {
    pub name: String,
    pub address: String,
    pub port: u16,
    /// Optional Ed25519 fingerprint (first 16 hex chars of the node's public
    /// key). When supplied at add-time, mutual-trust steps can short-circuit
    /// fingerprint TOFU.
    #[serde(default)]
    pub fingerprint: Option<String>,
}

/// Add a node to the registry manually and persist the registry to disk.
///
/// The node is inserted with `NodeState::Offline` so it renders greyed-out
/// until discovery confirms it online.
#[tauri::command]
pub async fn add_node(state: State<'_, AppState>, node: AddNodeInput) -> Result<(), AppError> {
    // Input validation
    if node.name.trim().is_empty() {
        return Err(AppError::InvalidInput("node name must not be empty".into()));
    }
    if node.address.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "node address must not be empty".into(),
        ));
    }
    if node.port == 0 {
        return Err(AppError::InvalidInput("port must be non-zero".into()));
    }

    let address: IpAddr = node
        .address
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("invalid IP address: {}", node.address)))?;

    let new_node = Node {
        name: node.name.clone(),
        address,
        port: node.port,
        state: NodeState::Offline,
        source: DiscoverySource::Static,
        fingerprint: node.fingerprint.clone(),
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
pub async fn remove_node(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidInput("node name must not be empty".into()));
    }

    let mut registry = state.registry.write().await;
    let removed = registry.remove(&name);
    if removed.is_none() {
        return Err(AppError::NotFound(format!("node '{}' not found", name)));
    }
    info!(%name, "node removed");

    if let Err(e) = registry.save() {
        warn!(error = %e, "failed to persist node registry");
    }

    Ok(())
}

/// List all nodes in the registry (online and offline).
#[tauri::command]
pub async fn list_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, AppError> {
    let registry = state.registry.read().await;
    Ok(registry.all().into_iter().cloned().collect())
}
