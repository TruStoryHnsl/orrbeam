use crate::{GpuInfo, MonitorInfo, Platform, PlatformError, PlatformInfo, ServiceInfo, ServiceStatus};
use orrbeam_core::Config;
use std::process::Command;

pub struct MacOsPlatform;

impl MacOsPlatform {
    pub fn new() -> Self {
        Self
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

impl Platform for MacOsPlatform {
    fn info(&self) -> PlatformInfo {
        let os_version = Self::run("sw_vers", &["-productVersion"]).ok();

        PlatformInfo {
            os: "macos".to_string(),
            os_version,
            display_server: Some("quartz".to_string()),
            hostname: hostname::get()
                .ok()
                .and_then(|h: std::ffi::OsString| h.into_string().ok())
                .unwrap_or_default(),
        }
    }

    fn sunshine_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError> {
        let path = config
            .sunshine_path
            .clone()
            .or_else(|| which::which("sunshine").ok().map(|p| p.to_string_lossy().to_string()))
            .or_else(|| {
                let app_path = "/Applications/Sunshine.app/Contents/MacOS/sunshine";
                std::path::Path::new(app_path)
                    .exists()
                    .then(|| app_path.to_string())
            });

        let running = Self::run("pgrep", &["-x", "sunshine"]).is_ok();

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
        let path = config.moonlight_path.clone().or_else(|| {
            let app_path = "/Applications/Moonlight.app";
            std::path::Path::new(app_path)
                .exists()
                .then(|| app_path.to_string())
        });

        let running = Self::run("pgrep", &["-f", "Moonlight"]).is_ok();

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
        let path = config
            .sunshine_path
            .as_deref()
            .unwrap_or("sunshine");

        Command::new(path).spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PlatformError::NotFound(path.to_string())
            } else {
                PlatformError::Io(e)
            }
        })?;
        Ok(())
    }

    fn stop_sunshine(&self) -> Result<(), PlatformError> {
        Self::run("pkill", &["-x", "sunshine"])?;
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
        let path = config
            .moonlight_path
            .as_deref()
            .unwrap_or("/Applications/Moonlight.app/Contents/MacOS/Moonlight");

        let mut cmd = Command::new(path);
        cmd.arg("stream").arg(address).arg(app);

        if windowed {
            cmd.arg("--display-mode").arg("windowed");
        }
        if let Some(res) = resolution {
            cmd.arg("--resolution").arg(res);
        }

        cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PlatformError::NotFound(path.to_string())
            } else {
                PlatformError::Io(e)
            }
        })?;
        Ok(())
    }

    fn stop_moonlight(&self) -> Result<(), PlatformError> {
        let _ = Self::run("pkill", &["-f", "Moonlight"]);
        Ok(())
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, PlatformError> {
        let output = Self::run(
            "system_profiler",
            &["SPDisplaysDataType", "-json"],
        )?;

        // Basic parsing — extract display names
        let monitors = vec![MonitorInfo {
            name: "Built-in Display".to_string(),
            resolution: "default".to_string(),
            refresh_rate: None,
            primary: true,
        }];

        // TODO: parse JSON output for full monitor details
        let _ = output; // suppress warning until JSON parsing is implemented
        Ok(monitors)
    }

    fn pair_moonlight(
        &self,
        config: &Config,
        address: &str,
        pin: &str,
    ) -> Result<(), PlatformError> {
        let path = config
            .moonlight_path
            .as_deref()
            .unwrap_or("/Applications/Moonlight.app/Contents/MacOS/Moonlight");

        Command::new(path)
            .args(["pair", address, "--pin", pin])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PlatformError::NotFound(path.to_string())
                } else {
                    PlatformError::Io(e)
                }
            })?;
        Ok(())
    }

    fn gpu_info(&self) -> Result<GpuInfo, PlatformError> {
        let output = Self::run(
            "system_profiler",
            &["SPDisplaysDataType"],
        )?;

        let name = output
            .lines()
            .find(|l| l.contains("Chipset Model:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "Apple Silicon".to_string());

        Ok(GpuInfo {
            name,
            encoder: "VideoToolbox".to_string(),
            driver: None,
        })
    }
}
