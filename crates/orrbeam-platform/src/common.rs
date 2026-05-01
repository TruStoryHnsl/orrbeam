//! Helpers shared across platform implementations.

use crate::PlatformError;
use std::process::Child;
use std::sync::Mutex;

/// Handle tracking for a process orrbeam spawned itself.
///
/// Used by `start_*` to stash the `Child`, and by `stop_*` to kill it
/// directly instead of resorting to `pkill`/`killall`/`taskkill` name matching.
pub(crate) type ChildSlot = Mutex<Option<Child>>;

/// Resolve a binary path: normalize empty strings to "unset" and fall back
/// to `which` across a list of candidate names.
///
/// Empty-string config values (`moonlight_path: ''` in YAML) are treated as
/// unset — serde-yaml deserializes them to `Some("")`, which would otherwise
/// bypass the fallback and cause `Command::new("")` to fail with NotFound.
pub(crate) fn resolve_binary(
    configured: Option<&str>,
    candidates: &[&str],
) -> Result<String, PlatformError> {
    if let Some(p) = configured.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(p.to_string());
    }
    for name in candidates {
        if let Ok(path) = which::which(name) {
            return Ok(path.to_string_lossy().to_string());
        }
    }
    Err(PlatformError::NotFound(candidates.join(" / ")))
}

/// Stop a child process that orrbeam spawned.
///
/// - If `try_wait()` reports the process already exited, clears the slot and returns Ok.
/// - Otherwise calls `kill()` and reaps via `wait()`.
/// - Returns `Ok(false)` if the slot was empty (caller should use its fallback).
/// - Returns `Ok(true)` if a tracked child was stopped.
pub(crate) fn stop_tracked(slot: &ChildSlot) -> Result<bool, PlatformError> {
    // Mutex poison is a fatal bug, not a recoverable error. `expect` is
    // the correct Rust idiom here; suppress the overly broad lint.
    #[allow(clippy::expect_used)]
    let mut guard = slot.lock().expect("child slot poisoned");
    let Some(mut child) = guard.take() else {
        return Ok(false);
    };
    match child.try_wait() {
        Ok(Some(_)) => Ok(true), // already exited, slot cleared
        Ok(None) => {
            child.kill().map_err(PlatformError::Io)?;
            let _ = child.wait();
            Ok(true)
        }
        Err(e) => Err(PlatformError::Io(e)),
    }
}

/// Store a freshly-spawned `Child` in a slot, reaping any stale predecessor.
pub(crate) fn store_child(slot: &ChildSlot, child: Child) {
    #[allow(clippy::expect_used)]
    let mut guard = slot.lock().expect("child slot poisoned");
    if let Some(mut old) = guard.take() {
        // Reap any old handle that may still be in the slot from a previous spawn.
        if matches!(old.try_wait(), Ok(None)) {
            let _ = old.kill();
            let _ = old.wait();
        }
    }
    *guard = Some(child);
}
