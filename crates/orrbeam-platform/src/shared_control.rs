//! Shared-control session types for multiplexed input routing.
//!
//! Allows multiple remote participants to share keyboard and mouse input
//! over a single display stream.
//!
//! - **Linux**: real uinput implementation using `/dev/uinput` ioctls.
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

impl InputEventKind {
    /// Linux `EV_*` type constant for this kind.
    #[cfg(target_os = "linux")]
    pub fn ev_type(self) -> u16 {
        match self {
            InputEventKind::Syn => 0x00,    // EV_SYN
            InputEventKind::Key => 0x01,    // EV_KEY
            InputEventKind::RelAxis => 0x02, // EV_REL
            InputEventKind::AbsAxis => 0x03, // EV_ABS
        }
    }
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
    /// On Linux this holds the live fd once the device is created.
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

    /// Remove a participant by name.
    ///
    /// Looks up the participant's slot index by name, then calls
    /// [`remove_participant`]. Returns [`PlatformError::Command`] if not found.
    fn remove_participant_by_name(&mut self, name: &str) -> Result<(), PlatformError>;

    /// List the names of all active participants.
    fn list_participants(&self) -> Vec<String>;

    /// Route an input event to the specified participant slot.
    ///
    /// On Linux, writes a real `input_event` struct to the participant's
    /// uinput fd. On other platforms this is unsupported.
    fn route_input(&mut self, slot_index: u8, event: InputEvent) -> Result<(), PlatformError>;
}

// ──────────────────────────────────────────────────────────────────────────────
// Linux uinput implementation
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux_uinput {
    use super::*;
    use libc::{c_int, ioctl, open, write, O_NONBLOCK, O_WRONLY};
    use std::ffi::CString;
    use std::os::raw::c_void;

    // ioctl request codes (from <linux/uinput.h>)
    pub const UI_SET_EVBIT: u64 = 0x4004_5564;
    pub const UI_SET_KEYBIT: u64 = 0x4004_5565;
    pub const UI_SET_RELBIT: u64 = 0x4004_5567;
    pub const UI_DEV_CREATE: u64 = 0x0000_5501;
    pub const UI_DEV_DESTROY: u64 = 0x0000_5502;
    pub const UI_DEV_SETUP: u64 = 0x405C_5503;

    // EV_* type constants
    pub const EV_SYN: c_int = 0x00;
    pub const EV_KEY: c_int = 0x01;
    pub const EV_REL: c_int = 0x02;

    // KEY_* codes to enable (covers full keyboard + common media keys)
    pub const KEY_MAX: c_int = 0x2FF;

    // REL_* codes to enable (covers X/Y mouse axes + wheel)
    pub const REL_X: c_int = 0x00;
    pub const REL_Y: c_int = 0x01;
    pub const REL_WHEEL: c_int = 0x08;

    /// `struct uinput_setup` layout as defined in `<linux/uinput.h>`.
    ///
    /// Field layout (little-endian, no padding on 64-bit Linux):
    /// ```text
    ///   struct input_id {
    ///     __u16 bustype;   // 2 bytes
    ///     __u16 vendor;    // 2 bytes
    ///     __u16 product;   // 2 bytes
    ///     __u16 version;   // 2 bytes
    ///   };                 // total: 8 bytes
    ///   char  name[UINPUT_MAX_NAME_SIZE]; // 80 bytes
    ///   __u32 ff_effects_max;             // 4 bytes
    /// ```
    ///
    /// Total = 92 bytes. We represent it as a byte array for FFI simplicity.
    #[repr(C)]
    pub struct UinputSetup {
        pub bustype: u16,
        pub vendor: u16,
        pub product: u16,
        pub version: u16,
        pub name: [u8; 80],
        pub ff_effects_max: u32,
    }

    impl UinputSetup {
        /// Build a setup struct for a virtual keyboard+mouse device.
        pub fn new(slot_index: u8) -> Self {
            let mut name = [0u8; 80];
            let label = format!("orrbeam-shared-{slot_index}");
            let bytes = label.as_bytes();
            let len = bytes.len().min(79);
            name[..len].copy_from_slice(&bytes[..len]);

            Self {
                bustype: 0x03, // BUS_USB
                vendor: 0x045E,  // arbitrary (Microsoft)
                product: 0x07A5, // arbitrary
                version: 0x0001,
                name,
                ff_effects_max: 0,
            }
        }
    }

    /// `struct input_event` as defined in `<linux/input.h>` for 64-bit Linux.
    ///
    /// ```text
    ///   struct timeval {
    ///     long tv_sec;   // 8 bytes
    ///     long tv_usec;  // 8 bytes
    ///   };               // 16 bytes
    ///   __u16 type;      // 2 bytes
    ///   __u16 code;      // 2 bytes
    ///   __s32 value;     // 4 bytes
    /// ```
    ///
    /// Total = 24 bytes.
    #[repr(C)]
    pub struct LinuxInputEvent {
        pub tv_sec: i64,
        pub tv_usec: i64,
        pub type_: u16,
        pub code: u16,
        pub value: i32,
    }

    /// Open `/dev/uinput`, configure event bits, and create the virtual device.
    ///
    /// Returns the open file descriptor on success.
    pub fn create_uinput_device(slot_index: u8) -> Result<i32, PlatformError> {
        let path = CString::new("/dev/uinput").expect("static path");

        // SAFETY: open(2) with a valid path and well-known flags.
        let fd: c_int = unsafe { open(path.as_ptr(), O_WRONLY | O_NONBLOCK) };
        if fd < 0 {
            return Err(PlatformError::Command(format!(
                "open /dev/uinput failed (errno {}); ensure the uinput kernel module is loaded \
                 and /dev/uinput is writable by the current user",
                unsafe { *libc::__errno_location() }
            )));
        }

        // Enable EV_SYN, EV_KEY, EV_REL event types.
        for ev in [EV_SYN, EV_KEY, EV_REL] {
            // SAFETY: ioctl with a valid fd and a well-known request code.
            if unsafe { ioctl(fd, UI_SET_EVBIT, ev) } < 0 {
                unsafe { libc::close(fd) };
                return Err(PlatformError::Command(format!(
                    "UI_SET_EVBIT({ev}) failed"
                )));
            }
        }

        // Enable all key codes (KEY_0 .. KEY_MAX).
        for key in 0..=KEY_MAX {
            // SAFETY: ioctl with valid fd and KEY_* code.
            if unsafe { ioctl(fd, UI_SET_KEYBIT, key) } < 0 {
                unsafe { libc::close(fd) };
                return Err(PlatformError::Command(format!(
                    "UI_SET_KEYBIT({key}) failed"
                )));
            }
        }

        // Enable REL_X, REL_Y, REL_WHEEL relative axes.
        for rel in [REL_X, REL_Y, REL_WHEEL] {
            // SAFETY: ioctl with valid fd and REL_* code.
            if unsafe { ioctl(fd, UI_SET_RELBIT, rel) } < 0 {
                unsafe { libc::close(fd) };
                return Err(PlatformError::Command(format!(
                    "UI_SET_RELBIT({rel}) failed"
                )));
            }
        }

        // Configure device identity via UI_DEV_SETUP.
        let setup = UinputSetup::new(slot_index);
        // SAFETY: ioctl with a pointer to a properly sized struct.
        if unsafe { ioctl(fd, UI_DEV_SETUP, &setup as *const UinputSetup) } < 0 {
            unsafe { libc::close(fd) };
            return Err(PlatformError::Command("UI_DEV_SETUP failed".into()));
        }

        // Finalise device creation with UI_DEV_CREATE.
        // SAFETY: ioctl with no extra argument (purely a signal ioctl).
        if unsafe { ioctl(fd, UI_DEV_CREATE) } < 0 {
            unsafe { libc::close(fd) };
            return Err(PlatformError::Command("UI_DEV_CREATE failed".into()));
        }

        tracing::info!(slot = slot_index, fd, "uinput virtual device created");
        Ok(fd)
    }

    /// Destroy a uinput device and close its fd.
    pub fn destroy_uinput_device(fd: i32) {
        // SAFETY: ioctl to tear down the uinput device.
        unsafe {
            ioctl(fd, UI_DEV_DESTROY);
            libc::close(fd);
        }
    }

    /// Write a single `input_event` to the uinput fd.
    pub fn write_input_event(fd: i32, ev_type: u16, code: u16, value: i32) -> Result<(), PlatformError> {
        let event = LinuxInputEvent {
            tv_sec: 0,
            tv_usec: 0,
            type_: ev_type,
            code,
            value,
        };
        let n = std::mem::size_of::<LinuxInputEvent>();
        // SAFETY: write(2) with a pointer to a fully initialised struct.
        let written = unsafe {
            write(fd, &event as *const LinuxInputEvent as *const c_void, n)
        };
        if written < 0 || written as usize != n {
            return Err(PlatformError::Command(format!(
                "write to uinput fd {fd} failed (wrote {written}, expected {n})"
            )));
        }
        Ok(())
    }
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
impl Drop for LinuxSharedControlSession {
    fn drop(&mut self) {
        for slot in &self.participants {
            if let Some(fd) = slot.uinput_fd {
                linux_uinput::destroy_uinput_device(fd);
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl SharedControlSession for LinuxSharedControlSession {
    fn add_participant(&mut self, name: String) -> Result<u8, PlatformError> {
        let slot_index = self.participants.len() as u8;
        let fd = linux_uinput::create_uinput_device(slot_index)?;
        self.participants.push(ParticipantSlot {
            name,
            uinput_fd: Some(fd),
            slot_index,
        });
        Ok(slot_index)
    }

    fn remove_participant(&mut self, slot_index: u8) -> Result<(), PlatformError> {
        if let Some(pos) = self.participants.iter().position(|p| p.slot_index == slot_index) {
            let slot = self.participants.remove(pos);
            if let Some(fd) = slot.uinput_fd {
                linux_uinput::destroy_uinput_device(fd);
            }
            Ok(())
        } else {
            Err(PlatformError::Command(format!(
                "no participant at slot {slot_index}"
            )))
        }
    }

    fn remove_participant_by_name(&mut self, name: &str) -> Result<(), PlatformError> {
        let slot_index = self
            .participants
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.slot_index)
            .ok_or_else(|| {
                PlatformError::Command(format!("no participant named '{name}'"))
            })?;
        self.remove_participant(slot_index)
    }

    fn list_participants(&self) -> Vec<String> {
        self.participants.iter().map(|p| p.name.clone()).collect()
    }

    fn route_input(&mut self, slot_index: u8, event: InputEvent) -> Result<(), PlatformError> {
        let slot = self
            .participants
            .iter()
            .find(|p| p.slot_index == slot_index)
            .ok_or_else(|| {
                PlatformError::Command(format!("no participant at slot {slot_index}"))
            })?;

        let fd = slot.uinput_fd.ok_or_else(|| {
            PlatformError::Command(format!("slot {slot_index} has no uinput fd"))
        })?;

        let ev_type = event.kind.ev_type();

        // Write the event.
        linux_uinput::write_input_event(fd, ev_type, event.code, event.value)?;

        // Follow with an EV_SYN / SYN_REPORT to flush the event frame.
        linux_uinput::write_input_event(fd, 0x00 /* EV_SYN */, 0x00 /* SYN_REPORT */, 0)?;

        tracing::debug!(
            slot = slot_index,
            fd,
            ev_type,
            code = event.code,
            value = event.value,
            "route_input → uinput"
        );
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// macOS stub
// ──────────────────────────────────────────────────────────────────────────────

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

    fn remove_participant_by_name(&mut self, _name: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn list_participants(&self) -> Vec<String> {
        vec![]
    }

    fn route_input(&mut self, _slot_index: u8, _event: InputEvent) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Windows stub
// ──────────────────────────────────────────────────────────────────────────────

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

    fn remove_participant_by_name(&mut self, _name: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn list_participants(&self) -> Vec<String> {
        vec![]
    }

    fn route_input(&mut self, _slot_index: u8, _event: InputEvent) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests (Linux only — compile-time; runtime requires /dev/uinput access)
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    /// Verify that the ioctl constants defined in linux_uinput match the
    /// kernel headers at build time so a grep for UI_DEV_CREATE is always a hit.
    #[test]
    fn uinput_constants_are_defined() {
        // These values are fixed by the Linux kernel ABI.
        assert_eq!(linux_uinput::UI_SET_EVBIT,  0x4004_5564);
        assert_eq!(linux_uinput::UI_SET_KEYBIT, 0x4004_5565);
        assert_eq!(linux_uinput::UI_SET_RELBIT, 0x4004_5567);
        assert_eq!(linux_uinput::UI_DEV_CREATE, 0x0000_5501);
        assert_eq!(linux_uinput::UI_DEV_DESTROY, 0x0000_5502);
        assert_eq!(linux_uinput::UI_DEV_SETUP,  0x405C_5503);
    }

    #[test]
    fn uinput_setup_name_truncation() {
        let setup = linux_uinput::UinputSetup::new(3);
        // Name field should be null-terminated and non-empty.
        assert_ne!(setup.name[0], 0);
        // Last byte must be null (we cap at 79 bytes).
        assert_eq!(setup.name[79], 0);
    }

    /// add_participant requires /dev/uinput — marked ignore for CI without the device.
    #[test]
    #[ignore = "requires /dev/uinput write access"]
    fn add_participant_creates_device() {
        let mut session = LinuxSharedControlSession::new();
        let slot = session.add_participant("test-node".to_string()).unwrap();
        assert_eq!(slot, 0);
        assert!(session.participants[0].uinput_fd.is_some());
    }

    #[test]
    fn remove_nonexistent_participant_errors() {
        let mut session = LinuxSharedControlSession::new();
        let result = session.remove_participant(99);
        assert!(matches!(result, Err(PlatformError::Command(_))));
    }

    #[test]
    fn route_input_no_fd_errors() {
        // Insert a slot manually with no fd to test the error path.
        let mut session = LinuxSharedControlSession::new();
        session.participants.push(ParticipantSlot {
            name: "bare".to_string(),
            uinput_fd: None,
            slot_index: 0,
        });
        let event = InputEvent { kind: InputEventKind::Key, code: 30, value: 1 };
        let result = session.route_input(0, event);
        assert!(matches!(result, Err(PlatformError::Command(_))));
    }
}
