pub mod client;
pub mod mdns;
pub mod orrtellite;

use orrbeam_core::{Config, Node, NodeRegistry, NodeState};
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    Mdns(String),
    #[error("orrtellite error: {0}")]
    Orrtellite(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Manages node discovery across all sources.
pub struct DiscoveryManager {
    registry: Arc<RwLock<NodeRegistry>>,
    config: Config,
}

impl DiscoveryManager {
    pub fn new(config: Config, registry: Arc<RwLock<NodeRegistry>>) -> Self {
        Self { registry, config }
    }

    /// Start all enabled discovery backends.
    pub async fn start(&self) -> Result<(), DiscoveryError> {
        if self.config.mdns_enabled {
            tracing::info!("starting mDNS discovery");
            let registry = self.registry.clone();
            tokio::spawn(async move {
                if let Err(e) = mdns::browse(registry).await {
                    tracing::error!("mDNS browse error: {e}");
                }
            });
        }

        if self.config.orrtellite_enabled {
            tracing::info!("starting orrtellite discovery");
            let registry = self.registry.clone();
            let url = self.config.orrtellite_url.clone();
            let key = self.config.orrtellite_api_key.clone();
            tokio::spawn(async move {
                if let Err(e) = orrtellite::poll(registry, &url, &key).await {
                    tracing::error!("orrtellite poll error: {e}");
                }
            });
        }

        // Add static nodes
        {
            let mut reg: tokio::sync::RwLockWriteGuard<'_, NodeRegistry> = self.registry.write().await;
            for entry in &self.config.static_nodes {
                if let Ok(addr) = entry.address.parse::<IpAddr>() {
                    reg.upsert(Node {
                        name: entry.name.clone(),
                        address: addr,
                        port: 47782,
                        state: NodeState::Offline,
                        source: orrbeam_core::node::DiscoverySource::Static,
                        fingerprint: None,
                        sunshine_available: false,
                        moonlight_available: false,
                        os: None,
                        encoder: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Get a snapshot of the current registry.
    pub async fn nodes(&self) -> NodeRegistry {
        let guard: tokio::sync::RwLockReadGuard<'_, NodeRegistry> = self.registry.read().await;
        guard.clone()
    }
}
