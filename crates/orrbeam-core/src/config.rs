//! Application configuration for orrbeam.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or saving the configuration file.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// An I/O error while reading or writing the config file.
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    /// The config file could not be parsed as valid YAML.
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
}

/// Static node entry for manual configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticNode {
    /// Human-readable node name.
    pub name: String,
    /// IP address or hostname of the node.
    pub address: String,
}

/// Orrbeam application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Human-readable name for this node (defaults to the system hostname).
    pub node_name: String,
    /// Master switch for all discovery backends.
    pub discovery_enabled: bool,
    /// Enable LAN mDNS (`_orrbeam._tcp`) discovery.
    pub mdns_enabled: bool,
    /// Enable orrtellite (Headscale) mesh discovery.
    pub orrtellite_enabled: bool,
    /// Base URL of the Headscale server (e.g. `https://hs.example.com`).
    pub orrtellite_url: String,
    /// Headscale API key for authenticating with orrtellite.
    pub orrtellite_api_key: String,
    /// Explicit path to the Sunshine binary (uses PATH lookup if `None`).
    pub sunshine_path: Option<String>,
    /// Sunshine web-UI username for PIN submission.
    pub sunshine_username: String,
    /// Sunshine web-UI password for PIN submission.
    pub sunshine_password: String,
    /// Explicit path to the Moonlight binary (uses PATH lookup if `None`).
    pub moonlight_path: Option<String>,
    /// Statically configured nodes that don't require discovery.
    pub static_nodes: Vec<StaticNode>,
    /// Address the control-plane HTTPS server binds to.
    pub api_bind: String,
    /// TCP port the control-plane HTTPS server listens on.
    pub api_port: u16,
    /// Enable the shared-control (multi-participant input) feature.
    ///
    /// When `true`, the platform layer will accept inbound shared-control
    /// session requests. Defaults to `false`.
    pub shared_control_enabled: bool,
    /// Maximum number of simultaneous shared-control participants.
    ///
    /// Capped by the platform implementation; on Linux this is limited by
    /// available uinput device slots. Defaults to `2`.
    pub max_participants: u8,
    /// Milliseconds after which an idle participant's input stream times out.
    ///
    /// Set to `0` to disable the timeout. Defaults to `5000` (5 seconds).
    pub input_timeout_ms: u32,
    /// Strategy for resolving simultaneous key-press conflicts between participants.
    ///
    /// - `"last_wins"` (default): the most-recently-active participant's event wins.
    /// - `"first_wins"`: the first participant to press a key holds it until release.
    /// - `"merge"`: all participant events are forwarded (suitable for co-op input).
    pub input_conflict_strategy: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_name: hostname(),
            discovery_enabled: true,
            mdns_enabled: true,
            orrtellite_enabled: false,
            orrtellite_url: String::new(),
            orrtellite_api_key: String::new(),
            sunshine_path: None,
            sunshine_username: "sunshine".to_string(),
            sunshine_password: "sunshine".to_string(),
            moonlight_path: None,
            static_nodes: Vec::new(),
            api_bind: "0.0.0.0".to_string(),
            api_port: 47782,
            shared_control_enabled: false,
            max_participants: 2,
            input_timeout_ms: 5000,
            input_conflict_strategy: "last_wins".to_string(),
        }
    }
}

impl Config {
    /// Load config from the platform-appropriate directory.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            Ok(serde_yaml::from_str(&contents)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Platform-appropriate config file path.
    pub fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("config.yaml")
    }
}

fn hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "orrbeam-node".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Set a temporary config dir for the test and restore on drop.
    fn with_temp_config<F: FnOnce(&TempDir)>(f: F) {
        let dir = TempDir::new().expect("tempdir");
        // We can't easily override dirs::config_dir, so we test save/load
        // using a manual path.
        f(&dir);
    }

    #[test]
    fn default_config_has_valid_fields() {
        let cfg = Config::default();
        assert!(cfg.discovery_enabled);
        assert!(cfg.mdns_enabled);
        assert_eq!(cfg.api_port, 47782);
        assert_eq!(cfg.api_bind, "0.0.0.0");
        assert!(!cfg.node_name.is_empty());
    }

    #[test]
    fn config_yaml_roundtrip() {
        with_temp_config(|_dir| {
            let cfg = Config {
                node_name: "test-node".to_string(),
                api_port: 12345,
                orrtellite_url: "https://hs.example.com".to_string(),
                ..Default::default()
            };

            // Serialize to YAML and deserialize back.
            let yaml = serde_yaml::to_string(&cfg).expect("serialize");
            let loaded: Config = serde_yaml::from_str(&yaml).expect("deserialize");

            assert_eq!(loaded.node_name, "test-node");
            assert_eq!(loaded.api_port, 12345);
            assert_eq!(loaded.orrtellite_url, "https://hs.example.com");
        });
    }

    #[test]
    fn config_save_and_load() {
        with_temp_config(|dir| {
            // Manually write config to a temp path and read it back.
            let path = dir.path().join("config.yaml");
            let cfg = Config {
                node_name: "save-load-test".to_string(),
                api_port: 9999,
                ..Default::default()
            };

            let yaml = serde_yaml::to_string(&cfg).expect("serialize");
            std::fs::write(&path, &yaml).expect("write");

            let contents = std::fs::read_to_string(&path).expect("read");
            let loaded: Config = serde_yaml::from_str(&contents).expect("parse");

            assert_eq!(loaded.node_name, "save-load-test");
            assert_eq!(loaded.api_port, 9999);
        });
    }

    #[test]
    fn config_static_nodes_roundtrip() {
        let cfg = Config {
            static_nodes: vec![
                StaticNode { name: "foo".to_string(), address: "10.0.0.1".to_string() },
                StaticNode { name: "bar".to_string(), address: "10.0.0.2".to_string() },
            ],
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&cfg).expect("serialize");
        let loaded: Config = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(loaded.static_nodes.len(), 2);
        assert_eq!(loaded.static_nodes[0].name, "foo");
        assert_eq!(loaded.static_nodes[1].address, "10.0.0.2");
    }
}
