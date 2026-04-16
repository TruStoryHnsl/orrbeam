//! Shared-control session types for multiplexed input routing.
//!
//! Allows multiple remote participants to share keyboard and mouse input
//! over a single display stream.
//!
//! - **Linux**: stub using uinput file descriptors (full implementation pending).
//! - **macOS / Windows**: stubs returning [`PlatformError::Unsupported`].

use crate::PlatformError;

/// The kind of input event being routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventKind {
    /// Key press or release.
    Key,
    /// Relative axis movement (e.g. mouse delta).
    RelAxis,
    /// Absolute axis position (e.g. touch or tablet coordinates).
    AbsAxis,
    /// Sync event — marks the end of an input event frame.
    Syn,
}

/// A low-level input event to be routed to a participant's virtual device.
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    /// What type of input this event represents.
    pub kind: InputEventKind,
    /// Linux input event code (e.g. `KEY_A = 30`).
    pub code: u16,
    /// Event value: `1` = press / `0` = release for keys; delta for relative axes.
    pub value: i32,
}

/// A participant slot in a shared-control session.
#[derive(Debug)]
pub struct ParticipantSlot {
    /// Display name of the participant (node name or user-supplied label).
    pub name: String,
    /// Raw uinput file descriptor for this participant's virtual device.
    ///
    /// Stored as `Option<i32>` for cross-platform compatibility.
    /// On Linux this will hold a real uinput fd once full uinput support is wired.
    /// On other platforms it is always `None`.
    pub uinput_fd: Option<i32>,
    /// Zero-based slot index used for routing and priority.
    pub slot_index: u8,
}

/// Trait for managing a shared-control session.
///
/// A shared-control session allows multiple remote participants to contribute
/// keyboard and mouse input to a single hosted display stream.
pub trait SharedControlSession: Send + Sync {
    /// Add a participant to the session.
    ///
    /// Returns the assigned slot index on success, or
    /// [`PlatformError::Unsupported`] on platforms without uinput support.
    fn add_participant(&mut self, name: String) -> Result<u8, PlatformError>;

    /// Remove a participant by slot index.
    ///
    /// Returns [`PlatformError::Command`] if no participant exists at that index.
    fn remove_participant(&mut self, slot_index: u8) -> Result<(), PlatformError>;

    /// Route an input event to the specified participant slot.
    ///
    /// Current implementation is a stub that logs the event and returns `Ok(())`.
    /// A full implementation would write the event to the participant's uinput fd.
    fn route_input(&mut self, slot_index: u8, event: InputEvent) -> Result<(), PlatformError>;
}

/// Linux uinput-based shared-control session.
#[cfg(target_os = "linux")]
pub struct LinuxSharedControlSession {
    /// Active participant slots.
    pub participants: Vec<ParticipantSlot>,
}

#[cfg(target_os = "linux")]
impl LinuxSharedControlSession {
    /// Create a new empty shared-control session.
    pub fn new() -> Self {
        Self { participants: Vec::new() }
    }
}

#[cfg(target_os = "linux")]
impl Default for LinuxSharedControlSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl SharedControlSession for LinuxSharedControlSession {
    fn add_participant(&mut self, name: String) -> Result<u8, PlatformError> {
        let slot_index = self.participants.len() as u8;
        self.participants.push(ParticipantSlot {
            name,
            uinput_fd: None, // uinput fd allocation is a future implementation step
            slot_index,
        });
        Ok(slot_index)
    }

    fn remove_participant(&mut self, slot_index: u8) -> Result<(), PlatformError> {
        if let Some(pos) = self.participants.iter().position(|p| p.slot_index == slot_index) {
            self.participants.remove(pos);
            Ok(())
        } else {
            Err(PlatformError::Command(format!(
                "no participant at slot {slot_index}"
            )))
        }
    }

    fn route_input(&mut self, slot_index: u8, event: InputEvent) -> Result<(), PlatformError> {
        // Stub: a real implementation would write to the participant's uinput fd.
        tracing::debug!(
            slot = slot_index,
            kind = ?event.kind,
            code = event.code,
            value = event.value,
            "route_input stub"
        );
        Ok(())
    }
}

/// macOS stub — shared control is not supported on macOS.
#[cfg(target_os = "macos")]
pub struct MacOsSharedControlSession;

#[cfg(target_os = "macos")]
impl SharedControlSession for MacOsSharedControlSession {
    fn add_participant(&mut self, _name: String) -> Result<u8, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn remove_participant(&mut self, _slot_index: u8) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn route_input(&mut self, _slot_index: u8, _event: InputEvent) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

/// Windows stub — shared control is not supported on Windows.
#[cfg(target_os = "windows")]
pub struct WindowsSharedControlSession;

#[cfg(target_os = "windows")]
impl SharedControlSession for WindowsSharedControlSession {
    fn add_participant(&mut self, _name: String) -> Result<u8, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn remove_participant(&mut self, _slot_index: u8) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn route_input(&mut self, _slot_index: u8, _event: InputEvent) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn add_participant_assigns_slot_index() {
        let mut session = LinuxSharedControlSession::new();
        let slot = session.add_participant("orrpheus".to_string()).unwrap();
        assert_eq!(slot, 0);
        let slot2 = session.add_participant("orrgate".to_string()).unwrap();
        assert_eq!(slot2, 1);
    }

    #[test]
    fn remove_existing_participant_ok() {
        let mut session = LinuxSharedControlSession::new();
        session.add_participant("alice".to_string()).unwrap();
        assert!(session.remove_participant(0).is_ok());
        assert!(session.participants.is_empty());
    }

    #[test]
    fn remove_nonexistent_participant_errors() {
        let mut session = LinuxSharedControlSession::new();
        let result = session.remove_participant(99);
        assert!(matches!(result, Err(PlatformError::Command(_))));
    }

    #[test]
    fn route_input_stub_returns_ok() {
        let mut session = LinuxSharedControlSession::new();
        session.add_participant("test".to_string()).unwrap();
        let event = InputEvent { kind: InputEventKind::Key, code: 30, value: 1 };
        assert!(session.route_input(0, event).is_ok());
    }
}
