//! Integration tests for orrbeam-platform.
//!
//! These tests verify the Platform trait's process lifecycle management
//! using mock/stub binaries in a temp directory.
//!
//! Real-hardware tests (actual GPU detection, actual Sunshine/Moonlight) are
//! marked `#[ignore]` and must be run explicitly with `cargo test -- --ignored`.

use orrbeam_core::config::Config;
use orrbeam_platform::ServiceStatus;
use std::fs;
use std::os::unix::fs::PermissionsExt;

// ---------------------------------------------------------------------------
// Mock binary helper
// ---------------------------------------------------------------------------

/// Creates a temporary directory containing mock `sunshine` and `moonlight`
/// shell scripts that immediately exit 0 (simulating installed-but-not-running
/// services). Returns the temp dir path so it can be added to PATH.
fn make_mock_bin_dir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("temp dir");

    // sunshine: prints a version string (like `sunshine --version` would) and exits 0
    let sunshine = dir.path().join("sunshine");
    fs::write(
        &sunshine,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"sunshine 0.99.0-mock\"; fi\nexit 0\n",
    )
    .expect("write sunshine mock");
    fs::set_permissions(&sunshine, fs::Permissions::from_mode(0o755)).expect("chmod sunshine");

    // moonlight-qt: same pattern
    let moonlight = dir.path().join("moonlight-qt");
    fs::write(
        &moonlight,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"moonlight-qt 5.0.0-mock\"; fi\nexit 0\n",
    )
    .expect("write moonlight mock");
    fs::set_permissions(&moonlight, fs::Permissions::from_mode(0o755)).expect("chmod moonlight");

    dir
}

/// Add `dir` to the front of PATH for this process.
fn prepend_path(dir: &std::path::Path) {
    let current = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.to_str().unwrap(), current);
    // SAFETY: test-only, single-threaded setup phase
    unsafe { std::env::set_var("PATH", new_path) };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// sunshine_status returns a ServiceInfo (no error) when `sunshine` is on PATH.
#[test]
fn sunshine_status_with_mock_binary_does_not_error() {
    let mock_dir = make_mock_bin_dir();
    prepend_path(mock_dir.path());

    let platform = orrbeam_platform::get_platform();
    let config = Config::default();
    let result = platform.sunshine_status(&config);

    // It may return NotInstalled or Installed depending on the implementation's
    // process-check strategy, but it must not return an Err for a detectable binary.
    assert!(
        result.is_ok(),
        "sunshine_status should not error when binary is present: {:?}",
        result
    );
}

/// moonlight_status returns a ServiceInfo (no error) when `moonlight-qt` is on PATH.
#[test]
fn moonlight_status_with_mock_binary_does_not_error() {
    let mock_dir = make_mock_bin_dir();
    prepend_path(mock_dir.path());

    let platform = orrbeam_platform::get_platform();
    let config = Config::default();
    let result = platform.moonlight_status(&config);

    assert!(
        result.is_ok(),
        "moonlight_status should not error when binary is present: {:?}",
        result
    );
}

/// stop_sunshine returns Ok (or a known error) — must not panic.
#[test]
fn stop_sunshine_does_not_panic() {
    let platform = orrbeam_platform::get_platform();
    // It is fine for this to return an error (Sunshine isn't running),
    // but it must not panic.
    let _ = platform.stop_sunshine();
}

/// stop_moonlight returns Ok (or a known error) — must not panic.
#[test]
fn stop_moonlight_does_not_panic() {
    let platform = orrbeam_platform::get_platform();
    let _ = platform.stop_moonlight();
}

/// platform.info() always returns a non-empty OS string.
#[test]
fn platform_info_os_is_nonempty() {
    let platform = orrbeam_platform::get_platform();
    let info = platform.info();
    assert!(!info.os.is_empty(), "os field must not be empty");
    assert!(!info.hostname.is_empty(), "hostname must not be empty");
}

/// gpu_info() must not panic (may return an error if nvidia-smi is absent).
#[test]
fn gpu_info_does_not_panic() {
    let platform = orrbeam_platform::get_platform();
    let _ = platform.gpu_info();
}

/// monitors() must not panic (may return empty vec if no display server).
#[test]
fn monitors_does_not_panic() {
    let platform = orrbeam_platform::get_platform();
    let _ = platform.monitors();
}

// ---------------------------------------------------------------------------
// Real-hardware tests (ignored by default)
// ---------------------------------------------------------------------------

/// Requires actual Sunshine installation. Run with `cargo test -- --ignored`.
#[test]
#[ignore]
fn real_sunshine_status_installed() {
    let platform = orrbeam_platform::get_platform();
    let config = Config::default();
    let info = platform.sunshine_status(&config).expect("sunshine_status");
    assert!(
        matches!(
            info.status,
            ServiceStatus::Running | ServiceStatus::Installed
        ),
        "expected Running or Installed, got {:?}",
        info.status
    );
}

/// Requires actual Moonlight installation.
#[test]
#[ignore]
fn real_moonlight_status_installed() {
    let platform = orrbeam_platform::get_platform();
    let config = Config::default();
    let info = platform
        .moonlight_status(&config)
        .expect("moonlight_status");
    assert!(
        matches!(
            info.status,
            ServiceStatus::Running | ServiceStatus::Installed
        ),
        "expected Running or Installed, got {:?}",
        info.status
    );
}
