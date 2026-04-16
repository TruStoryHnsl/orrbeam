//! Platform abstraction for Sunshine and Moonlight process management.
//!
//! Provides the [`Platform`] trait with OS-specific implementations for Linux,
//! macOS, and Windows.  Use [`get_platform`] to obtain the correct implementation
//! for the current target at runtime.

#![warn(missing_docs)]

mod common;
mod detect;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use detect::{GpuInfo, MonitorInfo, PlatformInfo, ServiceInfo, ServiceStatus};

use std::sync::Arc;

use orrbeam_core::Config;
use thiserror::Error;

/// Errors that can occur during platform operations (Sunshine / Moonlight process management).
#[derive(Error, Debug)]
pub enum PlatformError {
    /// A spawned command returned a non-zero exit code or unexpected output.
    #[error("command failed: {0}")]
    Command(String),
    /// The required binary was not found on PATH or at the configured path.
    #[error("binary not found: {0}")]
    NotFound(String),
    /// An I/O error occurred while spawning or communicating with a process.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The requested operation is not supported on this platform.
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
pub fn get_platform() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxPlatform::new())
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOsPlatform::new())
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsPlatform::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("unsupported platform")
    }
}
