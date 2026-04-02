mod detect;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use detect::{GpuInfo, MonitorInfo, PlatformInfo, ServiceInfo, ServiceStatus};

use orrbeam_core::Config;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("command failed: {0}")]
    Command(String),
    #[error("binary not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported platform")]
    Unsupported,
}

/// Platform-agnostic interface for managing Sunshine and Moonlight.
pub trait Platform: Send + Sync {
    /// Detect platform info (OS, GPU, display server).
    fn info(&self) -> PlatformInfo;

    /// Check Sunshine installation and status.
    fn sunshine_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError>;

    /// Check Moonlight installation and status.
    fn moonlight_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError>;

    /// Start Sunshine hosting.
    fn start_sunshine(&self, config: &Config) -> Result<(), PlatformError>;

    /// Stop Sunshine hosting.
    fn stop_sunshine(&self) -> Result<(), PlatformError>;

    /// Start Moonlight connection to a remote host.
    fn start_moonlight(
        &self,
        config: &Config,
        address: &str,
        app: &str,
        windowed: bool,
        resolution: Option<&str>,
    ) -> Result<(), PlatformError>;

    /// Stop active Moonlight connection.
    fn stop_moonlight(&self) -> Result<(), PlatformError>;

    /// List connected monitors.
    fn monitors(&self) -> Result<Vec<MonitorInfo>, PlatformError>;

    /// Detect GPU and encoder capabilities.
    fn gpu_info(&self) -> Result<GpuInfo, PlatformError>;

    /// Initiate Moonlight pairing with a remote Sunshine host using a predetermined PIN.
    fn pair_moonlight(
        &self,
        config: &Config,
        address: &str,
        pin: &str,
    ) -> Result<(), PlatformError>;
}

/// Get the platform implementation for the current OS.
pub fn get_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsPlatform::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPlatform::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("unsupported platform")
    }
}
