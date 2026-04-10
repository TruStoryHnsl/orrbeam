use mdns_sd::{ServiceDaemon, ServiceEvent};
use orrbeam_core::node::{DiscoverySource, Node, NodeRegistry, NodeState};
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVICE_TYPE: &str = "_orrbeam._tcp.local.";

/// Browse for orrbeam nodes on the local network via mDNS.
pub async fn browse(registry: Arc<RwLock<NodeRegistry>>) -> Result<(), crate::DiscoveryError> {
    let mdns = ServiceDaemon::new().map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?;

    let receiver = mdns
        .browse(SERVICE_TYPE)
        .map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?;

    // Process mDNS events in a loop
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let name = info
                        .get_property_val_str("node_name")
                        .unwrap_or_else(|| info.get_hostname().trim_end_matches('.'))
                        .to_string();

                    if let Some(addr) = info.get_addresses().iter().next() {
                        let node = Node {
                            name: name.clone(),
                            address: *addr,
                            port: info.get_port(),
                            state: NodeState::Online,
                            source: DiscoverySource::Mdns,
                            fingerprint: info
                                .get_property_val_str("fingerprint")
                                .map(String::from),
                            sunshine_available: info
                                .get_property_val_str("sunshine")
                                .is_some_and(|v| v == "true"),
                            moonlight_available: info
                                .get_property_val_str("moonlight")
                                .is_some_and(|v| v == "true"),
                            os: info.get_property_val_str("os").map(String::from),
                            encoder: info.get_property_val_str("encoder").map(String::from),
                            cert_sha256: None,
                        };

                        let reg = registry.clone();
                        tokio::runtime::Handle::current().block_on(async {
                            reg.write().await.upsert(node);
                        });

                        tracing::info!("mDNS: discovered node {name}");
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    let name = fullname.split('.').next().unwrap_or(&fullname).to_string();
                    let reg = registry.clone();
                    tokio::runtime::Handle::current().block_on(async {
                        reg.write().await.remove(&name);
                    });
                    tracing::info!("mDNS: node {name} removed");
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?;

    Ok(())
}
