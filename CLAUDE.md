# Orrbeam

Unified Sunshine/Moonlight mesh — bidirectional remote desktop nodes managed through a single desktop application.

## What it does

Every machine runs BOTH Sunshine (host) AND Moonlight (client), managed by a single application. The app presents a unified node list — click any node to connect TO it, or see who's connected to YOU. The mesh is bidirectional and self-organizing.

## Architecture

**Tauri v2 + Rust + React** — single self-contained GUI application. No daemon, no CLI.

```
orrbeam/
├── Cargo.toml                 # Workspace root
├── src-tauri/                 # Tauri v2 app (commands, AppState, entry point)
│   ├── src/
│   │   ├── main.rs            # Entry point (Wayland workaround, calls lib::run())
│   │   ├── lib.rs             # AppState, discovery init, command registration
│   │   └── commands/          # Tauri IPC: sunshine, moonlight, discovery, platform, settings
│   ├── tauri.conf.json        # App config (port 1421, window 1100x700)
│   └── capabilities/          # Permission capabilities (desktop, mobile)
├── crates/
│   ├── orrbeam-core/          # Config (YAML), Identity (Ed25519), Node/NodeRegistry types
│   ├── orrbeam-net/           # mDNS discovery + orrtellite (Headscale API) polling
│   └── orrbeam-platform/      # Platform abstraction: Sunshine/Moonlight process mgmt
│       └── src/               # linux.rs, macos.rs, windows.rs — trait Platform impls
├── frontend/                  # React 19 + TypeScript + Zustand + Tailwind CSS + Vite
│   └── src/
│       ├── api/tauri.ts       # IPC wrapper (lazy-loads @tauri-apps/api, mock mode for dev)
│       ├── stores/            # Zustand stores: sunshine, moonlight, platform
│       └── components/        # layout/, sunshine/, moonlight/, mesh/, ui/
└── v1/                        # Archived Python version (daemon + CLI + TUI)
```

## Quick start

```bash
# Development (frontend hot-reload + Rust backend)
cd frontend && npm install
cd .. && cargo tauri dev

# Production build
cargo tauri build
```

## Key design decisions

- **No daemon**: GUI manages Sunshine/Moonlight processes directly via `std::process::Command`
- **No CLI**: GUI is the sole interface
- **No headless mode**: Desktop-only application (no orrgate, no servers)
- **Side-by-side layout**: Left = Sunshine (host), Right = Moonlight (client), Bottom = mesh bar
- **Workspace crates**: Core logic in library crates, Tauri app is a thin command layer

## Tech stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust (Tauri v2) |
| Frontend | React 19, TypeScript, Zustand, Tailwind CSS |
| Build | Vite 6, Cargo workspace |
| Identity | Ed25519 (ed25519-dalek) |
| Discovery | mDNS (mdns-sd) + orrtellite (Headscale API via reqwest) |
| Config | YAML (~/.config/orrbeam/config.yaml) |

## Ports

- **1421** — Vite dev server (frontend)
- **47782** — (legacy) orrbeam daemon port, unused in v2
- **47984-47990** — Sunshine streaming
- **48010** — Sunshine RTSP

## Node discovery

1. **orrtellite mesh** — Headscale API at `orrtellite_url` with `orrtellite_api_key`, polls every 30s
2. **LAN mDNS** — `_orrbeam._tcp` service type
3. **Static entries** — config.yaml `static_nodes` list

No tailscale CLI dependency. Self-contained.

## Platform support

| Machine | OS | Host (Sunshine) | Client (Moonlight) |
|---------|-----|-----------------|-------------------|
| orrion | CachyOS (Linux) | NVENC (RTX 3070) | moonlight-qt |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app |
| Windows | Windows 10/11 | NVENC/AMF | Moonlight |
| iOS/iPad | iOS | N/A (client only) | Moonlight |
| Android | Android | N/A (client only) | Moonlight |

## Dev notes

- `cargo tauri dev` runs both Rust backend and Vite frontend with hot-reload
- Frontend mock mode: open `http://localhost:1421` in a browser (no Tauri) — mock data returned
- Platform crate uses `#[cfg(target_os)]` to compile only the current platform's implementation
- `AppState` holds `Arc<RwLock<Config>>`, `Identity`, `Arc<RwLock<NodeRegistry>>`, `Box<dyn Platform>`
- moonlight-qt `--version` opens GUI — use package manager for version detection
- Linux Wayland: main.rs sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` for WebKitGTK compatibility
