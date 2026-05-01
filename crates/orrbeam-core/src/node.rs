//! Mesh node types and the in-memory node registry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use thiserror::Error;
use time::OffsetDateTime;

/// Current state of a node in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeState {
    /// Node is reachable and not currently streaming.
    Online,
    /// Node is not reachable or has not been seen recently.
    Offline,
    /// Node is hosting a stream (Sunshine is active and a client is connected).
    Hosting,
    /// This node is connected to a remote stream (Moonlight is active).
    Connected,
}

/// How a node was discovered and added to the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoverySource {
    /// Discovered via LAN mDNS (`_orrbeam._tcp`).
    Mdns,
    /// Discovered via the orrtellite (Headscale) mesh API.
    Orrtellite,
    /// Manually configured in `config.yaml` as a static node.
    Static,
}

/// A node in the orrbeam mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Human-readable node name.
    pub name: String,
    /// IP address of the node's control-plane interface.
    pub address: IpAddr,
    /// TCP port of the node's control-plane HTTPS server.
    pub port: u16,
    /// Current reachability and streaming state.
    pub state: NodeState,
    /// How this node was discovered.
    pub source: DiscoverySource,
    /// Ed25519 fingerprint of the node (first 16 hex chars of public key).
    pub fingerprint: Option<String>,
    /// Whether Sunshine (remote-desktop host) is available on this node.
    pub sunshine_available: bool,
    /// Whether Moonlight (remote-desktop client) is available on this node.
    pub moonlight_available: bool,
    /// Operating system identifier (e.g. `"linux"`, `"macos"`, `"windows"`).
    pub os: Option<String>,
    /// Hardware encoder name if known (e.g. `"nvenc"`, `"videotoolbox"`).
    pub encoder: Option<String>,
    /// SHA-256 fingerprint of the node's TLS certificate (hex).
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

/// Errors that can occur while loading or saving the node registry.
#[derive(Debug, Error)]
pub enum NodeRegistryError {
    /// An I/O error while reading or writing the registry file.
    #[error("failed to read node registry: {0}")]
    Read(#[from] std::io::Error),
    /// The registry file could not be parsed as valid YAML.
    #[error("failed to parse node registry: {0}")]
    Parse(#[from] serde_yaml::Error),
}

/// Registry of all known nodes.
///
/// Nodes are keyed by name. The registry can be persisted to
/// `~/.config/orrbeam/known_nodes.yaml` so that previously-seen nodes
/// survive application restarts, even when they are currently offline.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NodeRegistry {
    nodes: HashMap<String, Node>,
}

impl NodeRegistry {
    /// Create a new empty registry.
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

    /// Path to the persistent node registry file:
    /// `~/.config/orrbeam/known_nodes.yaml`.
    pub fn persistence_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("known_nodes.yaml")
    }

    /// Alias for [`Self::persistence_path`]. Retained for callers using the
    /// older naming.
    pub fn default_path() -> PathBuf {
        Self::persistence_path()
    }

    /// Load the registry from disk at the default `persistence_path()`.
    /// Returns an empty registry (not an error) if the file does not exist yet.
    pub fn load() -> Result<Self, NodeRegistryError> {
        Self::load_from(Self::persistence_path())
    }

    /// Load the registry from a YAML file at `path`. Returns an empty
    /// registry if the file does not exist.
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Result<Self, NodeRegistryError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    /// Save the registry to disk at the default `persistence_path()`.
    pub fn save(&self) -> Result<(), NodeRegistryError> {
        self.save_to(Self::persistence_path())
    }

    /// Save the registry to a YAML file at `path`, creating parent directories
    /// as needed.
    pub fn save_to(&self, path: impl AsRef<std::path::Path>) -> Result<(), NodeRegistryError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Add a node manually (used by UI). Marks state as `Offline` since
    /// reachability is unknown until discovery confirms it.
    pub fn add_manual(&mut self, node: Node) {
        self.nodes.insert(node.name.clone(), node);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(name: &str, online: bool) -> Node {
        Node {
            name: name.to_string(),
            address: "127.0.0.1".parse().unwrap(),
            port: 47782,
            state: if online {
                NodeState::Online
            } else {
                NodeState::Offline
            },
            source: DiscoverySource::Static,
            fingerprint: None,
            sunshine_available: true,
            moonlight_available: true,
            os: None,
            encoder: None,
            cert_sha256: None,
            last_seen: None,
        }
    }

    #[test]
    fn upsert_and_get() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("alpha", true));
        let n = reg.get("alpha").expect("alpha present");
        assert_eq!(n.name, "alpha");
    }

    #[test]
    fn upsert_stamps_last_seen() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("alpha", true));
        let n = reg.get("alpha").unwrap();
        assert!(n.last_seen.is_some(), "upsert must stamp last_seen");
    }

    #[test]
    fn remove_cleans_up() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("beta", true));
        assert!(reg.get("beta").is_some());
        reg.remove("beta");
        assert!(reg.get("beta").is_none());
    }

    #[test]
    fn all_is_sorted_by_name() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("zeta", true));
        reg.upsert(make_node("alpha", true));
        reg.upsert(make_node("mu", true));
        let names: Vec<&str> = reg.all().iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["alpha", "mu", "zeta"]);
    }

    #[test]
    fn online_filters_offline() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("a", true));
        reg.upsert(make_node("b", false));
        let online = reg.online();
        assert_eq!(online.len(), 1);
        assert_eq!(online[0].name, "a");
    }

    #[test]
    fn online_count_matches_online() {
        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("x", true));
        reg.upsert(make_node("y", false));
        reg.upsert(make_node("z", true));
        assert_eq!(reg.online_count(), 2);
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("known_nodes.yaml");

        let mut reg = NodeRegistry::new();
        reg.upsert(make_node("persist-me", true));

        reg.save_to(&path).unwrap();
        let loaded = NodeRegistry::load_from(&path).unwrap();
        assert!(loaded.get("persist-me").is_some());
    }
}
