//! Network and discovery layer for the orrbeam mesh.
//!
//! This crate provides:
//! - [`DiscoveryManager`] — orchestrates mDNS and orrtellite node discovery.
//! - [`client`] — signed HTTPS client for node-to-node control-plane calls.
//! - [`server`] — axum-based TLS control-plane server with Ed25519 auth middleware.
//! - [`mdns`] — mDNS browse/register using `_orrbeam._tcp`.
//! - [`orrtellite`] — Headscale API polling for mesh node discovery.

#![warn(missing_docs)]

pub mod client;
pub mod mdns;
pub mod orrtellite;
pub mod server;

use mdns_sd::ServiceDaemon;
use orrbeam_core::{Config, Node, NodeRegistry, NodeState};
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during node discovery.
#[derive(Error, Debug)]
pub enum DiscoveryError {
    /// An mDNS operation failed.
    #[error("mDNS error: {0}")]
    Mdns(String),
    /// An orrtellite (Headscale) operation failed.
    #[error("orrtellite error: {0}")]
    Orrtellite(String),
    /// A network request failed.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Parameters needed to register this node as an mDNS service so peers can
/// discover it on the LAN.
pub struct RegistrationInfo {
    /// Ed25519 public-key fingerprint (hex-encoded).
    pub fingerprint: String,
    /// SHA-256 of the node's TLS certificate (hex-encoded).
    pub cert_sha256: String,
    /// Whether Sunshine (host) is available on this node.
    pub sunshine_available: bool,
    /// Whether Moonlight (client) is available on this node.
    pub moonlight_available: bool,
    /// Operating system identifier (e.g. `"linux"`, `"macos"`, `"windows"`).
    pub os: String,
    /// Hardware encoder name, if known (e.g. `"nvenc"`, `"videotoolbox"`).
    pub encoder: Option<String>,
    /// Control-plane port this node is listening on.
    pub port: u16,
}

/// Manages node discovery across all sources.
pub struct DiscoveryManager {
    registry: Arc<RwLock<NodeRegistry>>,
    config: Config,
    /// Keeps the mDNS registration alive for the lifetime of the manager.
    /// Dropping the inner `ServiceDaemon` would unregister the service.
    _mdns_registration: Option<ServiceDaemon>,
}

impl DiscoveryManager {
    /// Create a new [`DiscoveryManager`] bound to `config` and `registry`.
    pub fn new(config: Config, registry: Arc<RwLock<NodeRegistry>>) -> Self {
        Self {
            registry,
            config,
            _mdns_registration: None,
        }
    }

    /// Start all enabled discovery backends.
    ///
    /// If `registration` is `Some` and mDNS is enabled, this node will also
    /// advertise itself on the LAN so peers can discover it via mDNS.
    pub async fn start(&mut self, registration: Option<RegistrationInfo>) -> Result<(), DiscoveryError> {
        if self.config.mdns_enabled {
            tracing::info!("starting mDNS discovery");
            let registry = self.registry.clone();
            tokio::spawn(async move {
                if let Err(e) = mdns::browse(registry).await {
                    tracing::error!("mDNS browse error: {e}");
                }
            });

            if let Some(ref reg_info) = registration {
                tracing::info!("registering this node via mDNS as '{}'", self.config.node_name);
                let daemon = mdns::register(&self.config.node_name, reg_info)?;
                self._mdns_registration = Some(daemon);
            }
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
                        cert_sha256: None,
                        last_seen: None,
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
