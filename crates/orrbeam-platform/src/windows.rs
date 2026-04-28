//! Windows platform implementation for Sunshine/Moonlight process management.
//!
//! ## Binary discovery
//!
//! Sunshine on Windows installs to `%PROGRAMFILES%\Sunshine\sunshine.exe` by default
//! (LizardByte installer); moonlight-qt installs to `%PROGRAMFILES%\Moonlight Game
//! Streaming Client\Moonlight.exe`. Neither installer adds itself to PATH, so
//! `which::which("sunshine")` returns NotFound on a fresh install. We extend the
//! candidate list with absolute fallbacks via `windows_install_candidates()`.
//!
//! ## Monitor enumeration
//!
//! Uses Win32 `EnumDisplayMonitors` + `GetMonitorInfoW` from the `windows` crate.
//! Refresh rate is fetched via `EnumDisplaySettingsW` (DEVMODE.dmDisplayFrequency).
//!
//! ## GPU detection
//!
//! Probes (in order): `nvidia-smi` (NVENC), `wmic path Win32_VideoController` for
//! "AMD"/"Radeon" (AMF) and "Intel" (QSV). Falls back to "software" if no
//! hardware encoder is detected.

use crate::common::{ChildSlot, resolve_binary, stop_tracked, store_child};
use crate::{
    GpuInfo, MonitorInfo, Platform, PlatformError, PlatformInfo, ServiceInfo, ServiceStatus,
};
use orrbeam_core::Config;
use std::process::Command;

const SUNSHINE_CANDIDATES: &[&str] = &["sunshine", "sunshine.exe"];
const MOONLIGHT_CANDIDATES: &[&str] = &["Moonlight", "Moonlight.exe", "moonlight-qt"];

/// Build absolute-path fallbacks for Sunshine + Moonlight on Windows.
///
/// LizardByte's Sunshine installer and the moonlight-qt installer do NOT
/// add their bin dirs to PATH, so `which` lookups fail on a fresh install.
/// These candidates cover both per-machine and per-user installs.
fn windows_install_candidates(binary: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = binary.to_lowercase();

    let program_files = std::env::var("PROGRAMFILES").ok();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").ok();
    let local_appdata = std::env::var("LOCALAPPDATA").ok();

    if lower.contains("sunshine") {
        for root in [&program_files, &program_files_x86, &local_appdata]
            .iter()
            .copied()
            .flatten()
        {
            out.push(format!(r"{root}\Sunshine\sunshine.exe"));
        }
    } else if lower.contains("moonlight") {
        // The official MoonlightGameStreamingProject installer drops it at
        // `Moonlight Game Streaming\Moonlight.exe` (no "Client" — that was
        // an older name on the Github releases page).
        for root in [&program_files, &program_files_x86]
            .iter()
            .copied()
            .flatten()
        {
            out.push(format!(r"{root}\Moonlight Game Streaming\Moonlight.exe"));
            // Older "Client" suffixed install path (kept as fallback).
            out.push(format!(
                r"{root}\Moonlight Game Streaming Client\Moonlight.exe"
            ));
        }
        if let Some(local) = &local_appdata {
            out.push(format!(r"{local}\Programs\Moonlight\Moonlight.exe"));
        }
    }

    out
}

/// Resolve a Windows binary, falling back to absolute install paths if `which` fails.
fn resolve_windows_binary(
    configured: Option<&str>,
    candidates: &[&str],
) -> Result<String, PlatformError> {
    // First try the standard PATH-based resolution.
    if let Ok(path) = resolve_binary(configured, candidates) {
        return Ok(path);
    }

    // Fall back to checking absolute install paths.
    if let Some(first) = candidates.first() {
        for abs in windows_install_candidates(first) {
            if std::path::Path::new(&abs).is_file() {
                return Ok(abs);
            }
        }
    }

    Err(PlatformError::NotFound(candidates.join(" / ")))
}

/// Run `binary --version` and return the trimmed first line.
fn detect_version(path: &str) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    s.lines().next().map(|l| l.trim().to_string())
}

/// Windows implementation of [`Platform`].
pub struct WindowsPlatform {
    sunshine_child: ChildSlot,
    moonlight_child: ChildSlot,
}

impl WindowsPlatform {
    /// Create a new [`WindowsPlatform`] with empty process slots.
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

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
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
        let path =
            resolve_windows_binary(config.sunshine_path.as_deref(), SUNSHINE_CANDIDATES).ok();

        let running = Self::run("tasklist", &["/FI", "IMAGENAME eq sunshine.exe"])
            .map(|out| out.contains("sunshine.exe"))
            .unwrap_or(false);

        let version = path.as_deref().and_then(detect_version);

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
        let path =
            resolve_windows_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES).ok();

        let running = Self::run("tasklist", &["/FI", "IMAGENAME eq Moonlight.exe"])
            .map(|out| out.contains("Moonlight.exe"))
            .unwrap_or(false);

        // Moonlight-qt --version on Windows opens the GUI; do NOT call it.
        // Version detection deferred to the package manager / installer manifest.
        let version: Option<String> = None;

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
        let path = resolve_windows_binary(config.sunshine_path.as_deref(), SUNSHINE_CANDIDATES)?;

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
        let path = resolve_windows_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES)?;

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
        win32_monitors::enumerate()
    }

    fn pair_moonlight(
        &self,
        config: &Config,
        address: &str,
        pin: &str,
    ) -> Result<(), PlatformError> {
        let path = resolve_windows_binary(config.moonlight_path.as_deref(), MOONLIGHT_CANDIDATES)?;

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
        // 1. NVIDIA via nvidia-smi.
        if let Ok(output) = Self::run(
            "nvidia-smi",
            &["--query-gpu=name,driver_version", "--format=csv,noheader"],
        ) {
            let parts: Vec<&str> = output.split(", ").collect();
            return Ok(GpuInfo {
                name: parts.first().unwrap_or(&"NVIDIA GPU").to_string(),
                encoder: "nvenc".to_string(),
                driver: parts.get(1).map(|s| s.to_string()),
            });
        }

        // 2. AMD/Intel via wmic Win32_VideoController.
        if let Ok(out) = Self::run(
            "wmic",
            &["path", "Win32_VideoController", "get", "Name", "/value"],
        ) {
            let names: Vec<String> = out
                .lines()
                .filter_map(|l| l.trim().strip_prefix("Name="))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            for n in &names {
                let lower = n.to_lowercase();
                if lower.contains("amd") || lower.contains("radeon") {
                    return Ok(GpuInfo {
                        name: n.clone(),
                        encoder: "amf".to_string(),
                        driver: None,
                    });
                }
            }
            for n in &names {
                let lower = n.to_lowercase();
                if lower.contains("intel") {
                    return Ok(GpuInfo {
                        name: n.clone(),
                        encoder: "qsv".to_string(),
                        driver: None,
                    });
                }
            }
            if let Some(first) = names.first() {
                return Ok(GpuInfo {
                    name: first.clone(),
                    encoder: "software".to_string(),
                    driver: None,
                });
            }
        }

        Ok(GpuInfo {
            name: "Unknown".to_string(),
            encoder: "software".to_string(),
            driver: None,
        })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Win32 monitor enumeration
// ──────────────────────────────────────────────────────────────────────────────

mod win32_monitors {
    //! Real monitor enumeration via the `windows` crate.

    use crate::{MonitorInfo, PlatformError};
    use std::cell::RefCell;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        DEVMODEW, DISPLAY_DEVICE_PRIMARY_DEVICE, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors,
        EnumDisplaySettingsW, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    };

    thread_local! {
        static COLLECT: RefCell<Vec<MonitorInfo>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "system" fn monitor_enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        _lparam: LPARAM,
    ) -> BOOL {
        // Build a MONITORINFOEXW to get szDevice (display name).
        let mut info_ex = MONITORINFOEXW::default();
        info_ex.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: pointer cast between layout-compatible Win32 structs;
        // MONITORINFOEXW begins with a MONITORINFO field.
        let info_ptr = &mut info_ex as *mut MONITORINFOEXW as *mut MONITORINFO;
        let ok = unsafe { GetMonitorInfoW(hmon, info_ptr) };
        if !ok.as_bool() {
            return TRUE;
        }

        let name_wide = info_ex.szDevice;
        let name_len = name_wide
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(name_wide.len());
        let device_name = String::from_utf16_lossy(&name_wide[..name_len]);

        let primary = (info_ex.monitorInfo.dwFlags & DISPLAY_DEVICE_PRIMARY_DEVICE)
            == DISPLAY_DEVICE_PRIMARY_DEVICE;

        let rect = info_ex.monitorInfo.rcMonitor;
        let mut width = (rect.right - rect.left).max(0) as u32;
        let mut height = (rect.bottom - rect.top).max(0) as u32;
        let mut refresh: Option<u32> = None;

        let mut devmode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        // SAFETY: pcwstr from a null-terminated WCHAR slice we just decoded.
        let pcwstr = windows::core::PCWSTR(name_wide.as_ptr());
        let ok = unsafe { EnumDisplaySettingsW(pcwstr, ENUM_CURRENT_SETTINGS, &mut devmode) };
        if ok.as_bool() {
            if devmode.dmPelsWidth > 0 {
                width = devmode.dmPelsWidth;
            }
            if devmode.dmPelsHeight > 0 {
                height = devmode.dmPelsHeight;
            }
            if devmode.dmDisplayFrequency > 0 {
                refresh = Some(devmode.dmDisplayFrequency);
            }
        }

        COLLECT.with(|c| {
            c.borrow_mut().push(MonitorInfo {
                name: device_name,
                resolution: format!("{width}x{height}"),
                refresh_rate: refresh,
                primary,
            });
        });

        TRUE
    }

    pub(crate) fn enumerate() -> Result<Vec<MonitorInfo>, PlatformError> {
        COLLECT.with(|c| c.borrow_mut().clear());

        // SAFETY: EnumDisplayMonitors is a well-defined Win32 entry point
        // and our callback never panics across the FFI boundary.
        let ok = unsafe { EnumDisplayMonitors(None, None, Some(monitor_enum_proc), LPARAM(0)) };
        if !ok.as_bool() {
            return Err(PlatformError::Command(
                "EnumDisplayMonitors returned FALSE".into(),
            ));
        }

        let monitors = COLLECT.with(|c| c.borrow().clone());
        if monitors.is_empty() {
            return Ok(vec![MonitorInfo {
                name: "Primary".to_string(),
                resolution: "default".to_string(),
                refresh_rate: None,
                primary: true,
            }]);
        }
        Ok(monitors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_candidates_includes_program_files_for_sunshine() {
        // SAFETY: setting env in a test; std::env mutators are serialised by
        // the test harness when the same vars are touched.
        unsafe {
            std::env::set_var("PROGRAMFILES", r"C:\Program Files");
            std::env::set_var("ProgramFiles(x86)", r"C:\Program Files (x86)");
            std::env::set_var("LOCALAPPDATA", r"C:\Users\test\AppData\Local");
        }
        let cands = windows_install_candidates("sunshine");
        assert!(cands.iter().any(|c| c.contains(r"Sunshine\sunshine.exe")));
        assert!(cands.iter().any(|c| c.contains(r"C:\Program Files")));
    }

    #[test]
    fn install_candidates_includes_program_files_for_moonlight() {
        unsafe {
            std::env::set_var("PROGRAMFILES", r"C:\Program Files");
        }
        let cands = windows_install_candidates("Moonlight.exe");
        // Both the current path (no "Client") and the legacy path should appear.
        assert!(
            cands
                .iter()
                .any(|c| c.contains(r"Moonlight Game Streaming\Moonlight.exe")),
            "current install path missing; got: {cands:?}"
        );
        assert!(
            cands
                .iter()
                .any(|c| c.contains(r"Moonlight Game Streaming Client\Moonlight.exe")),
            "legacy install path missing; got: {cands:?}"
        );
    }

    #[test]
    fn install_candidates_unknown_binary_is_empty() {
        let cands = windows_install_candidates("unrelated.exe");
        assert!(cands.is_empty());
    }
}
