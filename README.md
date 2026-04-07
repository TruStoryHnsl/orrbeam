<div align="center">

<img src="branding/logo.png" alt="Orrbeam logo" width="160" />

# Orrbeam

**Unified Sunshine/Moonlight mesh — bidirectional remote desktop, one app.**

[![License: MIT](https://img.shields.io/badge/license-MIT-F8D808.svg)](#license)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-24C8DB.svg)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/rust-2024-CE422B.svg)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/react-19-61DAFB.svg)](https://react.dev/)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20macos%20%7C%20windows-787808.svg)](#platform-support)

</div>

---

Orrbeam is a desktop application that turns every machine you own into a node on a personal remote-desktop mesh. Each node runs **both** [Sunshine](https://app.lizardbyte.dev/Sunshine/) (host) and [Moonlight](https://moonlight-stream.org/) (client), and Orrbeam manages them through a single GUI. Click any node to stream **to** it; see at a glance who is streaming **from** you.

No daemon. No CLI. No central server. Just one app on every device, discovering each other over your LAN or [orrtellite](https://github.com/TruStoryHnsl/orrtellite) mesh VPN.

## Why Orrbeam

Sunshine and Moonlight are excellent on their own — but using them together across several machines means juggling two apps, two configurations, and two mental models per device. Orrbeam collapses that into one bidirectional view:

- **One app per machine.** Install Orrbeam, and that machine is automatically both a stream target *and* a stream source.
- **Unified node list.** All your machines appear in one panel, regardless of which side of the connection they're on.
- **Self-organizing mesh.** Nodes find each other via mDNS on the LAN and via the [orrtellite](https://github.com/TruStoryHnsl/orrtellite) Headscale API across networks. No manual peer lists to maintain.
- **Self-contained.** Ed25519 identity, YAML config, no cloud account, no telemetry.

## Features

- **Bidirectional control panel** — Sunshine (host) on the left, Moonlight (client) on the right, mesh status bar across the bottom.
- **Process supervision** — start, stop, and monitor Sunshine and Moonlight directly from the GUI; no shell required.
- **Cross-network discovery** — LAN discovery via `_orrbeam._tcp` mDNS, plus polling of an orrtellite/Headscale API for nodes outside the local network.
- **Static fallback** — pin nodes by hostname in `config.yaml` when discovery is not available.
- **Cryptographic identity** — each node generates an Ed25519 keypair on first run; no passwords or shared secrets.
- **Native, signed binaries** — distributed as a Tauri-built desktop app, not an Electron blob.
- **Mock-mode dev loop** — open the Vite frontend in a plain browser to iterate on UI without rebuilding the Rust shell.

## Screenshots

> Coming soon — the v2 GUI is under active development.

## Platform support

| Platform | Host (Sunshine) | Client (Moonlight) | Status |
|----------|-----------------|--------------------|--------|
| **Linux** (X11 / Wayland) | NVENC, VAAPI, software | `moonlight-qt` | Supported |
| **macOS** (Apple Silicon / Intel) | VideoToolbox | Moonlight.app | Supported |
| **Windows** 10 / 11 | NVENC, AMF, QuickSync | Moonlight | Supported |
| **iOS / iPadOS** | — | Moonlight | Client-only |
| **Android** | — | Moonlight | Client-only |

Sunshine and Moonlight must be installed separately — Orrbeam manages them, it does not bundle them. See the [Sunshine docs](https://docs.lizardbyte.dev/projects/sunshine/latest/) and [Moonlight downloads](https://moonlight-stream.org/) for platform-specific install instructions.

## Quick start

### Prerequisites

- **Rust** (stable, 2024 edition) — [install via rustup](https://rustup.rs/)
- **Node.js** 20+ and **npm**
- **Tauri v2 system dependencies** — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) for your OS
- **Sunshine** and/or **Moonlight** installed on the host

### Run from source

```bash
git clone https://github.com/TruStoryHnsl/orrbeam.git
cd orrbeam

# Install frontend dependencies
cd frontend && npm install && cd ..

# Launch the app in dev mode (Rust + Vite hot reload)
cargo tauri dev
```

### Build a release binary

```bash
cargo tauri build
```

The bundled installer/binary is written to `target/release/bundle/` (format depends on your OS — `.deb`, `.dmg`, `.msi`, `.AppImage`, etc.).

## Configuration

Orrbeam stores its config at:

| OS | Path |
|----|------|
| Linux | `~/.config/orrbeam/config.yaml` |
| macOS | `~/Library/Application Support/orrbeam/config.yaml` |
| Windows | `%APPDATA%\orrbeam\config.yaml` |

The file is created on first launch. A typical layout:

```yaml
# Display name shown to peers
node_name: orrion

# Optional orrtellite (Headscale) integration for off-LAN discovery
orrtellite_url: https://headscale.example.org
orrtellite_api_key: ${ORRTELLITE_API_KEY}   # read from env

# Manually pinned peers
static_nodes:
  - name: orrpheus
    address: 100.64.0.4
  - name: orrgate
    address: 192.168.1.145
```

Secrets such as `orrtellite_api_key` should be supplied via environment variables, never committed to the file directly.

## Architecture

Orrbeam is a single Tauri v2 desktop application backed by a small Rust workspace. The frontend talks to the backend exclusively through Tauri IPC commands.

```
orrbeam/
├── src-tauri/                 # Tauri v2 shell
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   ├── lib.rs             # AppState, command registration
│   │   └── commands/          # IPC: sunshine, moonlight, discovery, platform, settings
│   └── tauri.conf.json
├── crates/
│   ├── orrbeam-core/          # Config, Ed25519 identity, Node/NodeRegistry types
│   ├── orrbeam-net/           # mDNS discovery + Headscale API polling
│   └── orrbeam-platform/      # Platform abstraction (linux/macos/windows)
└── frontend/                  # React 19 + TypeScript + Zustand + Tailwind + Vite 6
    └── src/
        ├── api/tauri.ts       # IPC wrapper (with mock mode for browser dev)
        ├── stores/            # Zustand stores (sunshine, moonlight, platform)
        └── components/        # layout / sunshine / moonlight / mesh / ui
```

### Tech stack

| Layer        | Choice                                                       |
|--------------|--------------------------------------------------------------|
| Shell        | Tauri v2                                                     |
| Backend      | Rust 2024 edition, Tokio, `tracing`                          |
| Frontend     | React 19, TypeScript 5.8, Zustand, Tailwind CSS              |
| Build        | Vite 6, Cargo workspace                                      |
| Identity     | Ed25519 (`ed25519-dalek`)                                    |
| Discovery    | `mdns-sd` + Headscale REST API (`reqwest`)                   |
| Config       | YAML (`serde_yaml`)                                          |

### Network ports

| Port           | Purpose                                  |
|----------------|------------------------------------------|
| `1421`         | Vite dev server (development only)       |
| `47984–47990`  | Sunshine streaming (managed externally)  |
| `48010`        | Sunshine RTSP (managed externally)       |

## Development

```bash
# Run the full app with hot reload
cargo tauri dev

# Frontend-only iteration in a regular browser (uses mock IPC data)
cd frontend && npm run dev
# then open http://localhost:1421

# Frontend tests
cd frontend && npm test

# Rust tests
cargo test --workspace
```

The platform crate uses `#[cfg(target_os = "...")]` so only the current OS's implementation is compiled. When adding a feature that touches process management, add the matching method to the `Platform` trait and implement it for `linux.rs`, `macos.rs`, and `windows.rs`.

> **Linux / Wayland note:** `main.rs` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` to work around a WebKitGTK rendering bug. Remove only if you have verified the upstream fix is shipped.

## Project status

Orrbeam is in early `0.1.x` development. The Rust workspace, Tauri shell, mesh discovery, and platform abstractions are in place; the GUI is being built out in parallel. Expect breaking changes between minor versions until `1.0.0`.

A previous Python prototype (daemon + CLI + TUI) lives under `v1/` for reference. **All new work happens in the Tauri workspace** — `v1/` is read-only.

## Contributing

Contributions are welcome. Before opening a PR:

1. Run `cargo fmt --all` and `cargo clippy --workspace --all-targets`.
2. Run `cargo test --workspace` and `cd frontend && npm test`.
3. Use [Conventional Commits](https://www.conventionalcommits.org/) — e.g. `feat(mesh): add static node refresh`.
4. Keep changes focused; large refactors should be discussed in an issue first.

## License

Orrbeam is released under the [MIT License](https://opensource.org/licenses/MIT).
