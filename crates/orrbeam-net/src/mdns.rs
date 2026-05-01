//! LAN node discovery and registration via `_orrbeam._tcp` mDNS.

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use orrbeam_core::node::{DiscoverySource, Node, NodeRegistry, NodeState};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::RegistrationInfo;

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
                            fingerprint: info.get_property_val_str("fingerprint").map(String::from),
                            sunshine_available: info
                                .get_property_val_str("sunshine")
                                .is_some_and(|v| v == "true"),
                            moonlight_available: info
                                .get_property_val_str("moonlight")
                                .is_some_and(|v| v == "true"),
                            os: info.get_property_val_str("os").map(String::from),
                            encoder: info.get_property_val_str("encoder").map(String::from),
                            cert_sha256: info.get_property_val_str("cert_sha256").map(String::from),
                            last_seen: None,
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

/// Register this node as an `_orrbeam._tcp` mDNS service.
///
/// Publishes the node's name, control port, Ed25519 fingerprint, TLS cert
/// SHA-256, and capability flags as TXT properties so peers on the LAN can
/// discover and bootstrap trust.
///
/// `node_name` is the instance name used for the mDNS service record and
/// the `node_name` TXT property, typically sourced from [`Config::node_name`].
///
/// Returns the [`ServiceDaemon`] handle — keep it alive for the duration of
/// the app. Dropping it unregisters the service.
///
/// [`Config::node_name`]: orrbeam_core::Config
pub fn register(
    node_name: &str,
    info: &RegistrationInfo,
) -> Result<ServiceDaemon, crate::DiscoveryError> {
    let daemon = ServiceDaemon::new().map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?;

    // Resolve the local hostname for the mDNS SRV record.
    let host_name = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| node_name.to_string());
    // mdns-sd expects the hostname to end with a dot (fully-qualified).
    let fqhn = if host_name.ends_with('.') {
        host_name.clone()
    } else {
        format!("{host_name}.local.")
    };

    // Build TXT properties with all orrbeam service metadata.
    let port_str = info.port.to_string();
    let sunshine_str = if info.sunshine_available {
        "true"
    } else {
        "false"
    };
    let moonlight_str = if info.moonlight_available {
        "true"
    } else {
        "false"
    };

    let mut txt: Vec<(&str, &str)> = vec![
        ("node_name", node_name),
        ("fingerprint", &info.fingerprint),
        ("cert_sha256", &info.cert_sha256),
        ("sunshine", sunshine_str),
        ("moonlight", moonlight_str),
        ("os", &info.os),
        ("proto", "orrbeam/1"),
        ("control_port", &port_str),
    ];

    if let Some(enc) = info.encoder.as_deref() {
        txt.push(("encoder", enc));
    }

    let service_info = ServiceInfo::new(
        SERVICE_TYPE,
        node_name,
        &fqhn,
        "",
        info.port,
        txt.as_slice(),
    )
    .map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?
    .enable_addr_auto();

    daemon
        .register(service_info)
        .map_err(|e| crate::DiscoveryError::Mdns(e.to_string()))?;

    tracing::info!(
        "mDNS: registered service '{}' on port {} (fingerprint: {})",
        node_name,
        info.port,
        info.fingerprint
    );

    Ok(daemon)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `register` can create a daemon and register a service with
    /// valid parameters without panicking.
    ///
    /// Marked `#[ignore]` because mDNS registration requires a real network
    /// stack and socket access that is unavailable in most CI environments.
    #[test]
    #[ignore]
    fn register_returns_ok_with_valid_params() {
        let info = RegistrationInfo {
            fingerprint: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"
                .to_string(),
            cert_sha256: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                .to_string(),
            sunshine_available: true,
            moonlight_available: true,
            os: "linux".to_string(),
            encoder: Some("nvenc".to_string()),
            port: 47782,
        };
        let result = register("test-node", &info);
        assert!(
            result.is_ok(),
            "register() should succeed: {:?}",
            result.err()
        );
    }
}
