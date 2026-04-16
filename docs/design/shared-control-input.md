# Shared-Control Virtual Input — Design

**Status**: Spike / Pre-implementation  
**Date**: 2026-04-15  
**Scope**: Platform-specific virtual input device approach for shared-control mode

---

## Problem

Shared-control mode allows multiple remote participants to simultaneously control the host machine's input (keyboard + mouse). Each participant must receive a dedicated, isolated virtual input channel so that:

1. Input from participant A does not collide with participant B at the OS level.
2. The host user can revoke a participant's input without affecting others.
3. The system can route input to specific emulator/game player slots.

---

## Linux

### Mechanism: `uinput` (kernel module)

The Linux kernel's `uinput` module exposes `/dev/uinput`. A process with write access can create a new virtual input device that appears to the rest of the system as a real USB keyboard or mouse.

**Per-participant device creation:**
1. Open `/dev/uinput` (requires `CAP_NET_ADMIN` or membership in the `input` group, or a udev rule granting access to the orrbeam service user).
2. Call `UI_DEV_SETUP` ioctl to register device capabilities (EV_KEY, EV_REL, EV_SYN).
3. Call `UI_DEV_CREATE` to instantiate the device at `/dev/input/eventN`.
4. Write `input_event` structs to inject keystrokes and mouse movement.
5. On disconnect: call `UI_DEV_DESTROY` and close the fd.

**Rust crate**: [`evdev`](https://crates.io/crates/evdev) (v0.12+) provides safe uinput bindings via `VirtualDeviceBuilder`.

**Privilege requirement**: The uinput path must be accessible. Recommended: ship a udev rule:
```
KERNEL=="uinput", GROUP="input", MODE="0660"
```
And add the orrbeam service user to the `input` group.

**Player-slot routing**: For emulators (RetroArch, etc.) that enumerate joystick devices by `/dev/input/jsN` index, assign each participant a dedicated `BUS_USB` virtual gamepad with a unique `idVendor`/`idProduct` pair. The emulator picks up the new device automatically if hot-plug is enabled.

---

## macOS

### Mechanism: `IOHIDUserDevice` / `CGEvent`

macOS does not expose a direct uinput equivalent. Two approaches exist:

**Option A — `IOHIDUserDevice` (preferred, kernel-level)**:
- Create a `IOHIDUserDeviceRef` via `IOHIDUserDeviceCreate` with a property dict describing the device (Usage Page, Usage, Report Descriptor).
- Inject HID reports via `IOHIDUserDeviceHandleReport`.
- Each participant gets a separate `IOHIDUserDeviceRef` instance.
- **Entitlement**: Requires the `com.apple.security.temporary-exception.iokit-user-client-class` entitlement or the app must be signed with a com.apple.developer.hid.virtual.device entitlement (available to notarized apps).
- **Sandbox**: Not compatible with App Sandbox. Orrbeam already runs outside the sandbox (Tauri desktop app), so this is acceptable.

**Option B — `CGEvent` injection (simpler, less isolation)**:
- `CGEventCreateMouseEvent` / `CGEventCreateKeyboardEvent` + `CGEventPost(kCGSessionEventTap, ...)`.
- No per-participant isolation — all input goes to the same session event stream.
- **Entitlement**: Requires `com.apple.security.automation.apple-events` and Accessibility permission in System Settings → Privacy & Security.
- Suitable for single-participant mode only.

**Recommendation**: Use `IOHIDUserDevice` for multi-participant; fall back to `CGEvent` for single-participant where the HID entitlement is unavailable.

**Rust binding**: No stable crate exists. Use `objc2` or raw FFI to `IOKit.framework` and `CoreGraphics.framework`.

---

## Windows

### Mechanism: ViGEm Bus + `SendInput` fallback

**Option A — ViGEm Bus Driver (gamepad / joystick)**:
- [ViGEmBus](https://github.com/nefarius/ViGEmBus) is a kernel-mode virtual gamepad bus driver.
- The [ViGEmClient](https://github.com/nefarius/ViGEmClient) C API creates virtual Xbox 360 or DualShock 4 controllers.
- Each participant maps to one virtual gamepad instance.
- **Requirement**: ViGEmBus must be installed on the host (it is not included in Windows). Orrbeam installer must bundle it.
- Rust binding: [`vigem-client`](https://crates.io/crates/vigem-client).

**Option B — Interception Driver (keyboard + mouse)**:
- [Interception](https://github.com/oblitum/Interception) is a kernel-mode driver that intercepts and injects keyboard/mouse events at a lower level than `SendInput`.
- Per-device virtual instances; supports multiple simultaneous virtual keyboards/mice.
- **Requirement**: Signed kernel driver, requires test-signing or an EV-signed release.

**Option C — `SendInput` (fallback, no isolation)**:
- `SendInput(nInputs, pInputs, cbSize)` injects keyboard/mouse into the active foreground window.
- No per-participant isolation.
- Appropriate for single-participant only.

**Recommendation**: ViGEm for gamepad-focused shared control; Interception for keyboard+mouse multi-participant. `SendInput` as fallback if neither driver is present.

---

## Player-Slot Routing

When shared-control is used for multi-player gaming, each participant must map to a specific player slot:

1. **Slot assignment**: Server assigns slots 1–N at session start. Slots are deterministic (first-connected = player 1, etc.) and persisted in session state.
2. **Virtual device naming**: Each device is created with a display name of `"Orrbeam Player 1"`, `"Orrbeam Player 2"`, etc.
3. **Emulator hot-plug**: Emulators must have hot-plug enabled. If not supported, devices should be pre-created at session init rather than on first participant connect.
4. **Slot reclaim**: When a participant disconnects, their slot device is destroyed after a configurable grace period (`input_slot_release_delay_ms`, default 5000 ms), allowing brief disconnects to resume without re-assignment.

---

## Conflict Resolution

When two participants send contradictory input simultaneously (e.g., both press left and right arrow):

**Strategy: Last-Write-Wins (default)**
- The most recent `input_event` for a given key/axis wins.
- Simple to implement; natural for co-op where participants are taking turns.

**Strategy: Priority Queue (optional)**
- Each participant is assigned a priority (configurable, default equal).
- For conflicting events in the same 8 ms frame, the higher-priority participant's event is applied.
- Lower-priority participant's conflicting event is silently dropped.
- Suitable for "coach + student" scenarios.

Config field: `input_conflict_strategy: "last_write_wins" | "priority_queue"` (default: `"last_write_wins"`).

---

## Integration with Existing Moonlight/Sunshine Flow

Sunshine injects input from connected Moonlight clients via its own internal virtual input pipeline. For shared-control:

1. **Host-side**: Orrbeam creates N additional virtual input devices (one per shared-control participant).
2. **Sunshine**: Continues managing its own paired client's input via its existing path — shared-control participants are *additional* devices, not replacements.
3. **Participant input path**: Participant → Orrbeam control plane → virtual device on host → kernel input event queue.
4. **Not intercepting Sunshine**: We do not hook into Sunshine's internal input handling. This avoids coupling to Sunshine internals and works across Sunshine versions.

---

## Open Questions

- macOS entitlement process: needs testing on a real notarized build.
- Windows: determine whether ViGEm will be bundled in the installer or detected/prompted at runtime.
- Latency measurement: actual glass-to-glass latency of the virtual input path is unknown; benchmark needed (see INS-004).
