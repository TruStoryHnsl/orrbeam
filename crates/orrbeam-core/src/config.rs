use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
}

/// Static node entry for manual configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticNode {
    pub name: String,
    pub address: String,
}

/// Orrbeam application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub node_name: String,
    pub discovery_enabled: bool,
    pub mdns_enabled: bool,
    pub orrtellite_enabled: bool,
    pub orrtellite_url: String,
    pub orrtellite_api_key: String,
    pub sunshine_path: Option<String>,
    pub sunshine_username: String,
    pub sunshine_password: String,
    pub moonlight_path: Option<String>,
    pub static_nodes: Vec<StaticNode>,
    /// Address the control-plane HTTPS server binds to.
    pub api_bind: String,
    /// TCP port the control-plane HTTPS server listens on.
    pub api_port: u16,
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
