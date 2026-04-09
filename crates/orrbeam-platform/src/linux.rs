use crate::common::{resolve_binary, stop_tracked, store_child, ChildSlot};
use crate::{GpuInfo, MonitorInfo, Platform, PlatformError, PlatformInfo, ServiceInfo, ServiceStatus};
use orrbeam_core::Config;
use std::process::Command;

const SUNSHINE_CANDIDATES: &[&str] = &["sunshine"];
const MOONLIGHT_CANDIDATES: &[&str] = &["moonlight-qt", "moonlight"];

pub struct LinuxPlatform {
    sunshine_child: ChildSlot,
    moonlight_child: ChildSlot,
}

impl LinuxPlatform {
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

    fn display_server() -> Option<String> {
        std::env::var("XDG_SESSION_TYPE").ok()
    }
}

impl Platform for LinuxPlatform {
    fn info(&self) -> PlatformInfo {
        let os_version = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            });

        PlatformInfo {
            os: "linux".to_string(),
            os_version,
            display_server: Self::display_server(),
            hostname: hostname::get()
                .ok()
                .and_then(|h: std::ffi::OsString| h.into_string().ok())
                .unwrap_or_default(),
        }
    }

    fn sunshine_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError> {
        let path = resolve_binary(config.sunshine_path.as_deref(), SUNSHINE_CANDIDATES).ok();

        let running = Self::run("pgrep", &["-x", "sunshine"]).is_ok();

        let version = path.as_ref().and_then(|p| Self::run(p, &["--version"]).ok());

        Ok(ServiceInfo {
            name: "Sunshine".to_string(),
            status: if running {
                ServiceStatus::Running
            } else if path.is_some() {
                ServiceStatus::Installed
            } else {
                ServiceStatus::NotInstalled
            },
            version,
            path,
        })
    }

    fn moonlight_status(&self, config: &Config) -> Result<ServiceInfo, PlatformError> {
        let path = resolve_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES).ok();

        let running = Self::run("pgrep", &["-x", "moonlight-qt"])
            .or_else(|_| Self::run("pgrep", &["-x", "moonlight"]))
            .is_ok();

        // moonlight-qt --version opens GUI, so use package manager
        let version = Self::run("pacman", &["-Q", "moonlight-qt"])
            .ok()
            .and_then(|out| out.split_whitespace().nth(1).map(String::from))
            .or_else(|| {
                Self::run("dpkg-query", &["-W", "-f=${Version}", "moonlight-qt"]).ok()
            });

        Ok(ServiceInfo {
            name: "Moonlight".to_string(),
            status: if running {
                ServiceStatus::Running
            } else if path.is_some() {
                ServiceStatus::Installed
            } else {
                ServiceStatus::NotInstalled
            },
            version,
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
        // Primary: kill the handle we tracked at spawn time.
        if stop_tracked(&self.sunshine_child)? {
            return Ok(());
        }
        // Fallback: process was started outside orrbeam (e.g. systemd).
        // pkill returns nonzero if no match — treat that as "already stopped".
        let _ = Self::run("pkill", &["-x", "sunshine"]);
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
        let _ = Self::run("pkill", &["-x", "moonlight-qt"]);
        let _ = Self::run("pkill", &["-x", "moonlight"]);
        Ok(())
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, PlatformError> {
        let output = Self::run("xrandr", &["--listmonitors"])
            .or_else(|_| Self::run("wlr-randr", &[]))?;

        let monitors = output
            .lines()
            .filter(|l| l.contains('/'))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let primary = line.contains('*');
                let name = parts.last()?.to_string();
                let res = parts.iter().find(|p| p.contains('x')).map(|r| {
                    r.split('/').next().unwrap_or(*r).to_string()
                });
                Some(MonitorInfo {
                    name,
                    resolution: res.unwrap_or_default(),
                    refresh_rate: None,
                    primary,
                })
            })
            .collect();

        Ok(monitors)
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
        // Try nvidia-smi first, then vainfo
        if let Ok(output) = Self::run("nvidia-smi", &["--query-gpu=name,driver_version", "--format=csv,noheader"]) {
            let parts: Vec<&str> = output.split(", ").collect();
            return Ok(GpuInfo {
                name: parts.first().unwrap_or(&"NVIDIA GPU").to_string(),
                encoder: "NVENC".to_string(),
                driver: parts.get(1).map(|s| s.to_string()),
            });
        }

        if Self::run("vainfo", &[]).is_ok() {
            return Ok(GpuInfo {
                name: "Intel/AMD GPU".to_string(),
                encoder: "VAAPI".to_string(),
                driver: None,
            });
        }

        Ok(GpuInfo {
            name: "Unknown".to_string(),
            encoder: "Software".to_string(),
            driver: None,
        })
    }
}
