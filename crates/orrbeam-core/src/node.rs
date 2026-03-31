use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

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

    /// Insert or update a node.
    pub fn upsert(&mut self, node: Node) {
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
}
