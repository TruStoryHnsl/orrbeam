# Orrbeam v1 (Archived)

Python-based daemon + CLI + TUI for managing Sunshine/Moonlight. Superseded by v2 (Tauri + Rust + React desktop GUI).

**Do not modify** — this is a read-only archive for reference.

## What it was

- `orrbeam/daemon.py` — aiohttp REST API on port 47782
- `orrbeam/cli.py` — Click CLI (`orrbeam status`, `orrbeam connect`, etc.)
- `orrbeam/tui/` — Textual TUI
- `orrbeam/popup.py` — Hotkey-triggered overlay
- `orrbeam/platform/` — ABC platform layer (Linux + macOS)
- `orrbeam/discovery.py` — mDNS + orrtellite node discovery
- `orrbeam/identity.py` — Ed25519 keypair
- `apple/` — Planned iOS/iPad SwiftUI companion (never started)

## Why it was archived

User decided orrbeam should be a single self-contained desktop GUI, not a daemon/CLI stack. v2 reimplements the same concepts (platform detection, discovery, Sunshine/Moonlight process management, pairing) in Rust with a React frontend via Tauri v2.
