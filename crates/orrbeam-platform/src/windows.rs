use crate::common::{ChildSlot, resolve_binary, stop_tracked, store_child};
use crate::{
    GpuInfo, MonitorInfo, Platform, PlatformError, PlatformInfo, ServiceInfo, ServiceStatus,
};
use orrbeam_core::Config;
use std::process::Command;

const SUNSHINE_CANDIDATES: &[&str] = &["sunshine", "sunshine.exe"];
const MOONLIGHT_CANDIDATES: &[&str] = &["Moonlight", "Moonlight.exe", "moonlight-qt"];

pub struct WindowsPlatform {
    sunshine_child: ChildSlot,
    moonlight_child: ChildSlot,
}

impl WindowsPlatform {
    pub fn new() -> Self {
        Self {
            sunshine_child: ChildSlot::default(),
            moonlight_child: ChildSlot::default(),
        }
    }

    fn run(cmd: &str, args: &[&str]) -> Result<String, PlatformError> {
        match Command::new(cmd).args(args).output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    Err(PlatformError::Command(stderr))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(PlatformError::NotFound(cmd.to_string()))
            }
            Err(e) => Err(PlatformError::Io(e)),
        }
    }
}

impl Platform for WindowsPlatform {
    fn info(&self) -> PlatformInfo {
        let os_version = Self::run("cmd", &["/c", "ver"]).ok();

        PlatformInfo {
            os: "windows".to_string(),
            os_version,
            display_server: Some("win32".to_string()),
            hostname: hostname::get()
                .ok()
                .and_then(|h: std::ffi::OsString| h.into_string().ok())
                .unwrap_or_default(),
        }
    }

    fn sunshine_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError> {
        let path = resolve_binary(config.sunshine_path.as_deref(), SUNSHINE_CANDIDATES).ok();

        let running = Self::run("tasklist", &["/FI", "IMAGENAME eq sunshine.exe"])
            .map(|out| out.contains("sunshine.exe"))
            .unwrap_or(false);

        Ok(ServiceInfo {
            name: "Sunshine".to_string(),
            status: if running {
                ServiceStatus::Running
            } else if path.is_some() {
                ServiceStatus::Installed
            } else {
                ServiceStatus::NotInstalled
            },
            version: None,
            path,
        })
    }

    fn moonlight_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError> {
        let path = resolve_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES).ok();

        let running = Self::run("tasklist", &["/FI", "IMAGENAME eq Moonlight.exe"])
            .map(|out| out.contains("Moonlight.exe"))
            .unwrap_or(false);

        Ok(ServiceInfo {
            name: "Moonlight".to_string(),
            status: if running {
                ServiceStatus::Running
            } else if path.is_some() {
                ServiceStatus::Installed
            } else {
                ServiceStatus::NotInstalled
            },
            version: None,
            path,
        })
    }

    fn start_sunshine(&self, config: &Config) -> Result<(), PlatformError> {
        let path = resolve_binary(config.sunshine_path.as_deref(), SUNSHINE_CANDIDATES)?;

        let child = Command::new(&path).spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PlatformError::NotFound(path.clone())
            } else {
                PlatformError::Io(e)
            }
        })?;
        store_child(&self.sunshine_child, child);
        Ok(())
    }

    fn stop_sunshine(&self) -> Result<(), PlatformError> {
        if stop_tracked(&self.sunshine_child)? {
            return Ok(());
        }
        let _ = Self::run("taskkill", &["/IM", "sunshine.exe", "/F"]);
        Ok(())
    }

    fn start_moonlight(
        &self,
        config: &Config,
        address: &str,
        app: &str,
        windowed: bool,
        resolution: Option<&str>,
    ) -> Result<(), PlatformError> {
        let path = resolve_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES)?;

        let mut cmd = Command::new(&path);
        cmd.arg("stream").arg(address).arg(app);

        if windowed {
            cmd.arg("--display-mode").arg("windowed");
        }
        if let Some(res) = resolution {
            cmd.arg("--resolution").arg(res);
        }

        let child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PlatformError::NotFound(path.clone())
            } else {
                PlatformError::Io(e)
            }
        })?;
        store_child(&self.moonlight_child, child);
        Ok(())
    }

    fn stop_moonlight(&self) -> Result<(), PlatformError> {
        if stop_tracked(&self.moonlight_child)? {
            return Ok(());
        }
        let _ = Self::run("taskkill", &["/IM", "Moonlight.exe", "/F"]);
        Ok(())
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, PlatformError> {
        // TODO: Use win32 API for proper monitor enumeration
        Ok(vec![MonitorInfo {
            name: "Primary".to_string(),
            resolution: "default".to_string(),
            refresh_rate: None,
            primary: true,
        }])
    }

    fn pair_moonlight(
        &self,
        config: &Config,
        address: &str,
        pin: &str,
    ) -> Result<(), PlatformError> {
        let path = resolve_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES)?;

        Command::new(&path)
            .args(["pair", address, "--pin", pin])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PlatformError::NotFound(path.clone())
                } else {
                    PlatformError::Io(e)
                }
            })?;
        Ok(())
    }

    fn gpu_info(&self) -> Result<GpuInfo, PlatformError> {
        // Try nvidia-smi first
        if let Ok(output) = Self::run(
            "nvidia-smi",
            &["--query-gpu=name,driver_version", "--format=csv,noheader"],
        ) {
            let parts: Vec<&str> = output.split(", ").collect();
            return Ok(GpuInfo {
                name: parts.first().unwrap_or(&"NVIDIA GPU").to_string(),
                encoder: "NVENC".to_string(),
                driver: parts.get(1).map(|s| s.to_string()),
            });
        }

        Ok(GpuInfo {
            name: "Unknown".to_string(),
            encoder: "Software".to_string(),
            driver: None,
        })
    }
}
