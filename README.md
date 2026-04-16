# Orrbeam

Orrbeam is a bidirectional Sunshine/Moonlight mesh: every node is both a host and a client, and the desktop app manages the whole graph from one place. Instead of treating Sunshine and Moonlight as two separate tools with one-way sessions, Orrbeam presents your machines as peers that can discover each other, expose host controls, and launch client sessions through a single Tauri application.

## Feature Highlights

- Unified two-panel UI for Sunshine host controls and Moonlight client actions
- Node discovery over LAN mDNS and orrtellite-backed Headscale APIs
- Ed25519 node identity plus trusted-peer management for control-plane access
- Cross-platform Rust workspace with Linux, macOS, and planned Windows support
- Browser-mode frontend mocks for fast React iteration without rebuilding the Tauri shell

## Supported Platforms

| Machine | OS | Host (Sunshine) | Client (Moonlight) | Status |
| --- | --- | --- | --- | --- |
| orrion | CachyOS (Linux) | NVENC (RTX 3070) | moonlight-qt | Primary dev |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app | Primary dev |
| Windows | Windows 10/11 | NVENC/AMF | Moonlight | Planned |
| iPad/iPhone | iOS | N/A | Moonlight (via Tauri) | Planned |
| Android | Android | N/A | Moonlight (via Tauri) | Planned |

Mobile targets are client-only. The Sunshine panel is expected to stay hidden or disabled on iOS and Android builds.

## Prerequisites

- Rust 1.80+ with the standard `rustup` toolchain
- Node.js 20+ and npm
- Tauri v2 system dependencies for your OS
- Sunshine installed on machines that will host streams
- Moonlight installed on machines that will initiate client sessions
- Platform-compatible GPU encode support where required by Sunshine
  Linux commonly uses NVENC or VAAPI; macOS uses VideoToolbox; Windows uses NVENC, AMF, or QuickSync depending on hardware

## Quickstart

```bash
git clone git@github.com:TruStoryHnsl/orrbeam.git
cd orrbeam
cd frontend && npm install && cd ..
cargo tauri dev
```

Useful local commands:

```bash
cargo test --workspace
cd frontend && npm run test
```

## Configuration

Runtime configuration lives under the per-user `orrbeam` config directory:

- Linux: `~/.config/orrbeam/`
- macOS: `~/Library/Application Support/orrbeam/`
- Windows: `%APPDATA%\\orrbeam\\`

The app persists settings such as the node name, discovery toggles, static peers, and Sunshine/Moonlight binary paths in `config.yaml` within that directory.

## Architecture Overview

The repo is a Cargo workspace plus a React frontend:

- `crates/orrbeam-core`: config, identity, node models, peer models, TLS and wire helpers
- `crates/orrbeam-net`: discovery, control-plane client/server logic, nonce handling
- `crates/orrbeam-platform`: OS abstraction, hardware detection, Sunshine/Moonlight process management
- `src-tauri`: app state, IPC commands, tray integration, background tasks
- `frontend`: React 19, TypeScript, Zustand stores, Tauri API wrapper, panel UI

See [docs/architecture.md](docs/architecture.md) for the dependency graph, IPC surface, and data-flow details.

## Contributing

Contribution workflow, branch naming, commit conventions, and verification commands live in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Orrbeam is released under the [MIT License](LICENSE).
