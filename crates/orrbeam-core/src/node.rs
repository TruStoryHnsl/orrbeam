use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::OffsetDateTime;

/// Current state of a node in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeState {
    Online,
    Offline,
    Hosting,
    Connected,
}

/// Discovery source for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoverySource {
    Mdns,
    Orrtellite,
    Static,
}

/// A node in the orrbeam mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub state: NodeState,
    pub source: DiscoverySource,
    pub fingerprint: Option<String>,
    pub sunshine_available: bool,
    pub moonlight_available: bool,
    pub os: Option<String>,
    pub encoder: Option<String>,
    #[serde(default)]
    pub cert_sha256: Option<String>,
    /// Timestamp of the last time this node was seen on the mesh.
    /// `None` for nodes that have never been observed (manually added).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub last_seen: Option<OffsetDateTime>,
}

/// Errors produced by [`NodeRegistry`] persistence operations.
#[derive(Debug, Error)]
pub enum NodeRegistryError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Registry of all known nodes.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NodeRegistry {
    nodes: HashMap<String, Node>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a node, recording `last_seen` as now.
    pub fn upsert(&mut self, mut node: Node) {
        node.last_seen = Some(OffsetDateTime::now_utc());
        self.nodes.insert(node.name.clone(), node);
    }

    /// Remove a node by name.
    pub fn remove(&mut self, name: &str) -> Option<Node> {
        self.nodes.remove(name)
    }

    /// Get a node by name.
    pub fn get(&self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }

    /// All nodes as a sorted vec (by name).
    pub fn all(&self) -> Vec<&Node> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by_key(|n| &n.name);
        nodes
    }

    /// Nodes that are currently online.
    pub fn online(&self) -> Vec<&Node> {
        self.all()
            .into_iter()
            .filter(|n| n.state != NodeState::Offline)
            .collect()
    }

    /// Count of online nodes.
    pub fn online_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.state != NodeState::Offline)
            .count()
    }

    // ── Persistence ──────────────────────────────────────────────────────────

    /// Default path for the persistent registry: `~/.config/orrbeam/known_nodes.yaml`.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("orrbeam")
            .join("known_nodes.yaml")
    }

    /// Load the registry from a YAML file, returning an empty registry if the
    /// file does not exist.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, NodeRegistryError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let nodes: HashMap<String, Node> = serde_yaml::from_str(&contents)?;
        Ok(Self { nodes })
    }

    /// Save the registry to a YAML file, creating parent directories as needed.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), NodeRegistryError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_yaml::to_string(&self.nodes)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Mark a node offline (sets state to `Offline`) without removing it from
    /// the persistent store, so it continues to appear in the UI with a greyed
    /// status.
    pub fn mark_offline(&mut self, name: &str) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.state = NodeState::Offline;
        }
    }
}
