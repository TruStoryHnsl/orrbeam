//! Tauri IPC commands for shared-control session management.
//!
//! Shared control allows multiple remote participants to contribute keyboard
//! and mouse input to a single hosted display stream via uinput virtual
//! devices (Linux only).

use tauri::State;

use crate::AppState;
use crate::error::AppError;

// ---------------------------------------------------------------------------
// start_shared_control
// ---------------------------------------------------------------------------

/// Start a shared-control session on this node.
///
/// Idempotent — calling while a session is already active returns `Ok(())`.
/// On platforms other than Linux this returns `AppError::Unsupported`.
#[tauri::command]
pub async fn start_shared_control(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state
        .shared_control
        .lock()
        .map_err(|_| AppError::Internal("shared_control lock poisoned".into()))?;

    if guard.is_some() {
        // Already started — idempotent.
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        use orrbeam_platform::shared_control::LinuxSharedControlSession;
        *guard = Some(Box::new(LinuxSharedControlSession::new()));
        tracing::info!("shared-control session started (Linux/uinput)");
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(AppError::Unsupported(
            "shared control is only supported on Linux".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// stop_shared_control
// ---------------------------------------------------------------------------

/// Stop the active shared-control session, releasing all uinput devices.
///
/// Idempotent — calling when no session is active returns `Ok(())`.
#[tauri::command]
pub async fn stop_shared_control(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state
        .shared_control
        .lock()
        .map_err(|_| AppError::Internal("shared_control lock poisoned".into()))?;

    // Drop the session (this destroys all uinput devices via Drop impl).
    *guard = None;
    tracing::info!("shared-control session stopped");
    Ok(())
}

// ---------------------------------------------------------------------------
// add_sc_participant
// ---------------------------------------------------------------------------

/// Add a named participant to the active shared-control session.
///
/// Returns the assigned slot index (0-based). `name` must be 1–64 characters.
/// Returns `AppError::Unsupported` if no session is active.
#[tauri::command]
pub async fn add_sc_participant(state: State<'_, AppState>, name: String) -> Result<u8, AppError> {
    // Validate input (commercial scope).
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::InvalidInput(
            "participant name must be 1–64 characters".into(),
        ));
    }

    let mut guard = state
        .shared_control
        .lock()
        .map_err(|_| AppError::Internal("shared_control lock poisoned".into()))?;

    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Unsupported("shared control session not started".into()))?;

    let slot = session
        .add_participant(name.clone())
        .map_err(AppError::from)?;

    tracing::info!(name, slot, "shared-control participant added");
    Ok(slot)
}

// ---------------------------------------------------------------------------
// remove_sc_participant
// ---------------------------------------------------------------------------

/// Remove a participant from the shared-control session by name.
///
/// Looks up the participant's slot index from the active session, then removes
/// it and destroys the associated uinput device.
#[tauri::command]
pub async fn remove_sc_participant(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), AppError> {
    // Validate input.
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::InvalidInput(
            "participant name must be 1–64 characters".into(),
        ));
    }

    let mut guard = state
        .shared_control
        .lock()
        .map_err(|_| AppError::Internal("shared_control lock poisoned".into()))?;

    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Unsupported("shared control session not started".into()))?;

    session
        .remove_participant_by_name(&name)
        .map_err(|e| match e {
            orrbeam_platform::PlatformError::Command(msg) if msg.contains("no participant") => {
                AppError::NotFound(format!("participant '{name}' not found"))
            }
            other => AppError::from(other),
        })?;

    tracing::info!(name, "shared-control participant removed");
    Ok(())
}

// ---------------------------------------------------------------------------
// list_sc_participants
// ---------------------------------------------------------------------------

/// List the names of all active participants in the shared-control session.
///
/// Returns an empty list if no session is active.
#[tauri::command]
pub async fn list_sc_participants(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let guard = state
        .shared_control
        .lock()
        .map_err(|_| AppError::Internal("shared_control lock poisoned".into()))?;

    match guard.as_ref() {
        None => Ok(vec![]),
        Some(session) => Ok(session.list_participants()),
    }
}
