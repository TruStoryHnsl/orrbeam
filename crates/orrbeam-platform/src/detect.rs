use serde::{Deserialize, Serialize};

/// Status of a managed service (Sunshine or Moonlight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Installed,
    Running,
    NotInstalled,
    Unknown,
}

/// Information about a managed service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: ServiceStatus,
    pub version: Option<String>,
    pub path: Option<String>,
}

/// Platform information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_version: Option<String>,
    pub display_server: Option<String>,
    pub hostname: String,
}

/// Monitor/display information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub resolution: String,
    pub refresh_rate: Option<u32>,
    pub primary: bool,
}

/// GPU and encoder information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub encoder: String,
    pub driver: Option<String>,
}
