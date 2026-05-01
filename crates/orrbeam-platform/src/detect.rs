//! Platform detection types for Sunshine, Moonlight, GPU, and display info.

use serde::{Deserialize, Serialize};

/// Status of a managed service (Sunshine or Moonlight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// The service binary is installed but not currently running.
    Installed,
    /// The service is installed and actively running.
    Running,
    /// The service binary was not found on this system.
    NotInstalled,
    /// The service status could not be determined.
    Unknown,
}

/// Information about a managed service (Sunshine or Moonlight).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Human-readable service name (e.g. `"sunshine"`, `"moonlight-qt"`).
    pub name: String,
    /// Current status of the service.
    pub status: ServiceStatus,
    /// Version string reported by the binary, if available.
    pub version: Option<String>,
    /// Filesystem path to the installed binary.
    pub path: Option<String>,
}

/// Platform and environment information for this node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system identifier (e.g. `"linux"`, `"macos"`, `"windows"`).
    pub os: String,
    /// OS version string (e.g. `"Ubuntu 24.04"`, `"macOS 14.4"`).
    pub os_version: Option<String>,
    /// Display server in use on Linux (e.g. `"x11"`, `"wayland"`).
    pub display_server: Option<String>,
    /// System hostname.
    pub hostname: String,
}

/// Information about a connected monitor/display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Display connector name (e.g. `"DP-1"`, `"HDMI-A-1"`, `"Built-in Retina Display"`).
    pub name: String,
    /// Resolution string (e.g. `"3840x2160"`).
    pub resolution: String,
    /// Refresh rate in Hz, if known.
    pub refresh_rate: Option<u32>,
    /// Whether this is the primary display.
    pub primary: bool,
}

/// GPU and hardware encoder information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name (e.g. `"NVIDIA GeForce RTX 3070"`).
    pub name: String,
    /// Hardware encoder available on this GPU (e.g. `"nvenc"`, `"vaapi"`, `"videotoolbox"`).
    pub encoder: String,
    /// Driver version string, if available.
    pub driver: Option<String>,
}
