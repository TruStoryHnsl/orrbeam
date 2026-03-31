# Orrbeam — Feedback to Prompt Queue

---

## Entry: 2026-04-13 04:10 — Desktop GUI pivot

### Raw Input
ORRBEAM FEEDBACK

- I need a desktop UI that manages sunshine and moonlight in one two-parted standalone application.

### Optimized Prompt

Build a standalone Tauri v2 desktop application (Rust + React) that unifies Sunshine host management and Moonlight client management into a single side-by-side two-panel interface. No daemon, no CLI. Cross-platform: Linux, macOS, Windows, iOS, Android. Reuse existing node discovery (mDNS + orrtellite) and identity (Ed25519) concepts from v1.

### Resolved Questions
1. **GUI toolkit**: Tauri v2 + Rust backend + React frontend (matches concord v2 stack)
2. **Daemon**: Eliminated — GUI manages Sunshine/Moonlight processes directly
3. **CLI**: Eliminated — GUI is the sole interface
4. **Layout**: Side-by-side (left = Sunshine host, right = Moonlight client)
5. **Mesh visibility**: Both full mesh view AND personal status

### Status
Generated: 2026-04-13
Resolved: 2026-03-30
Executed: in_progress
