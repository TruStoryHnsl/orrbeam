use orrbeam_core::node::{DiscoverySource, Node, NodeRegistry, NodeState};
use serde::Deserialize;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

const POLL_INTERVAL_SECS: u64 = 30;
const ORRBEAM_PORT: u16 = 47782;

#[derive(Debug, Deserialize)]
struct HeadscaleNode {
    name: String,
    #[serde(rename = "ipAddresses")]
    ip_addresses: Vec<String>,
    online: bool,
}

#[derive(Debug, Deserialize)]
struct HeadscaleResponse {
    nodes: Vec<HeadscaleNode>,
}

/// Poll orrtellite (Headscale API) for mesh nodes.
pub async fn poll(
    registry: Arc<RwLock<NodeRegistry>>,
    url: &str,
    api_key: &str,
) -> Result<(), crate::DiscoveryError> {
    let client = reqwest::Client::new();

    loop {
        match fetch_nodes(&client, url, api_key).await {
            Ok(nodes) => {
                let mut reg = registry.write().await;
                for hs_node in nodes {
                    if let Some(addr) = hs_node
                        .ip_addresses
                        .iter()
                        .find_map(|ip| ip.parse::<IpAddr>().ok())
                    {
                        reg.upsert(Node {
                            name: hs_node.name.clone(),
                            address: addr,
                            port: ORRBEAM_PORT,
                            state: if hs_node.online {
                                NodeState::Online
                            } else {
                                NodeState::Offline
                            },
                            source: DiscoverySource::Orrtellite,
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
                tracing::debug!("orrtellite: updated node list");
            }
            Err(e) => {
                tracing::warn!("orrtellite poll failed: {e}");
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

async fn fetch_nodes(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
) -> Result<Vec<HeadscaleNode>, crate::DiscoveryError> {
    let resp = client
        .get(format!("{url}/api/v1/node"))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await?
        .json::<HeadscaleResponse>()
        .await?;

    Ok(resp.nodes)
}
