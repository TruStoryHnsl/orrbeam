# Orrbeam — Master Development Plan

A unified Sunshine/Moonlight mesh — bidirectional remote desktop nodes managed through a single desktop application.

## Architecture

### Core Concept

Traditional Sunshine+Moonlight: one machine runs Sunshine (host), another runs Moonlight (client). One-way tunnel. Each pair must be configured separately.

**Orrbeam**: Every machine runs BOTH Sunshine AND Moonlight, managed by a single application. The app presents a unified node list — click any node to connect TO it, or see who's connected to YOU. The mesh is bidirectional and self-organizing.

```
Traditional:
  Machine A (Sunshine) <── Machine B (Moonlight)
  One-way. B sees A's screen. A cannot see B.

Orrbeam:
  Machine A (Sunshine + Moonlight) <-> Machine B (Sunshine + Moonlight)
  Either can host. Either can connect. Roles are dynamic.
```

### Tech Stack (v2)

- **Backend**: Rust (Tauri v2)
- **Frontend**: React 19 + TypeScript + Zustand + Tailwind CSS
- **Build**: Vite + Cargo workspace
- **Platforms**: Linux, macOS, Windows, iOS, Android
- **No daemon, no CLI** — single self-contained GUI application

### Project Structure

```
orrbeam/
├── Cargo.toml                 # Workspace root
├── src-tauri/                 # Tauri v2 app
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs             # AppState + run()
│   │   └── commands/          # Tauri IPC commands
│   │       ├── mod.rs
│   │       ├── sunshine.rs
│   │       ├── moonlight.rs
│   │       ├── discovery.rs
│   │       ├── platform.rs
│   │       └── settings.rs
│   └── capabilities/
│       ├── default.json
│       └── mobile.json
├── crates/
│   ├── orrbeam-core/          # Types, config, identity (Ed25519)
│   ├── orrbeam-net/           # Discovery (mDNS, orrtellite)
│   └── orrbeam-platform/      # OS abstraction: Sunshine/Moonlight process mgmt
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   ├── tailwind.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── api/tauri.ts       # IPC wrapper + browser mocks
│       ├── stores/            # Zustand: sunshine, moonlight, discovery, settings
│       └── components/
│           ├── layout/        # Shell, sidebar, mesh overview
│           ├── sunshine/      # Left panel: host controls
│           ├── moonlight/     # Right panel: client controls
│           ├── mesh/          # Full mesh visualization
│           └── ui/            # Shared primitives
├── v1/                        # Archived Python version
├── CLAUDE.md
├── PLAN.md
└── .scope
```

### UI Layout

**Side-by-side two-panel design:**

```
┌─────────────────────────────────────────────────────────┐
│  Orrbeam                                    [mesh] [⚙]  │
├────────────────────────┬────────────────────────────────┤
│  ☀ SUNSHINE (Host)     │  🌙 MOONLIGHT (Client)        │
│                        │                                │
│  Status: ● Hosting     │  Status: ○ Disconnected        │
│  Encoder: NVENC        │                                │
│  Monitor: DP-1         │  Available Nodes:              │
│  Resolution: 2560x1440 │  ┌──────────────────────┐     │
│  FPS: 60               │  │ orrpheus (macOS)  ●  │     │
│                        │  │ mbp15 (Ubuntu)    ●  │     │
│  Connected Clients:    │  │ ipad-pro (iOS)    ○  │     │
│  ├─ orrpheus           │  └──────────────────────┘     │
│  └─ ipad-pro           │                                │
│                        │  [Connect to orrpheus]          │
│  [Stop Hosting]        │                                │
├────────────────────────┴────────────────────────────────┤
│  Mesh: 4 nodes online  orrion ←→ orrpheus ← ipad-pro   │
└─────────────────────────────────────────────────────────┘
```

- **Left panel (Sunshine)**: Host controls, encoder info, monitor selection, connected clients, start/stop
- **Right panel (Moonlight)**: Node browser, connect/disconnect, stream settings, resolution/mode
- **Bottom bar**: Full mesh visualization showing all nodes and their connections
- **Shared settings**: Identity, discovery config, network — accessible via gear icon

### Node Discovery (carried from v1)

Nodes find each other via (in priority order):
1. **orrtellite mesh** — Headscale API for mesh IPs (no Tailscale CLI)
2. **LAN mDNS** — `_orrbeam._tcp` service type
3. **Static entries** — config file

### Shared vs. Unique Parameters

**Shared (unified in settings):**
- Node identity (name, Ed25519 fingerprint)
- Discovery config (mDNS, orrtellite, static nodes)
- Network interface binding

**Sunshine-only (left panel):**
- Active monitor/display selection + rotation
- Encoder (NVENC/VAAPI/VideoToolbox), bitrate, codec
- Audio codec, session timeout
- Application list, client certificates

**Moonlight-only (right panel):**
- Target node + application selection
- Display mode (windowed/fullscreen)
- Requested resolution, input device mappings
- Latency/performance mode

### Target Platforms

| Machine | OS | Host (Sunshine) | Client (Moonlight) | Status |
|---------|-----|-----------------|-------------------|--------|
| orrion | CachyOS (Linux) | NVENC (RTX 3070) | moonlight-qt | Primary dev |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app | Primary dev |
| Windows | Windows 10/11 | NVENC/AMF | Moonlight | Planned |
| iPad/iPhone | iOS | N/A | Moonlight (via Tauri) | Planned |
| Android | Android | N/A | Moonlight (via Tauri) | Planned |

**Note**: Mobile platforms are client-only (Moonlight panel only, Sunshine panel hidden/disabled).

## Feature Roadmap

### v1 (archived — Python CLI/daemon/TUI)
- [x] Node identity — Ed25519 keypair generation
- [x] Platform abstraction layer (Linux + macOS)
- [x] Daemon + CLI + TUI + popup overlay
- [x] Archived to v1/

### v2 (current — Tauri desktop GUI)
1. [x] **Scaffold Tauri v2 workspace** — Cargo workspace + React frontend + crates
2. [x] **Platform crate** — Detect OS, GPU, Sunshine/Moonlight install status, process management
3. [x] **Core crate** — Config (YAML), identity (Ed25519), node types, sunshine.conf read/write
4. [x] **Net crate** — mDNS discovery + orrtellite polling
5. [x] **Two-panel layout** — Side-by-side Sunshine + Moonlight panels
6. [x] **Sunshine management** — Start/stop, monitor selector, codec/fps/bitrate config
7. [x] **Moonlight management** — Node browser, connect/disconnect, resolution/mode/app settings
8. [x] **Mesh visualization** — Bottom bar showing all nodes and connections
9. [x] **Pairing workflow** — Initiate (Moonlight PIN + subprocess) + Accept (Sunshine API) dialogs
10. [ ] **System tray** — Minimize to tray, quick connect shortcuts
11. [ ] **Mobile builds** — iOS + Android client-only mode
12. [ ] **Windows support** — Build + test on Windows

## Resolved Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| GUI toolkit | Tauri v2 + Rust + React | Cross-platform (desktop + mobile), lightweight, matches concord v2 stack |
| Daemon | Eliminated | No headless use case — orrbeam is a desktop app. GUI manages processes directly. |
| CLI | Eliminated | GUI is the sole interface |
| Layout | Side-by-side | Left = Sunshine (host), Right = Moonlight (client) |
| Mesh visibility | Both views | Full mesh + personal hosting/connection status |
| Headless (orrgate) | Not supported | Orrgate is a services VM — SSH suffices, no graphical desktop needed |

## Recent Changes
- 2026-04-01: **Item 9 done + interactivity bugfix.** Pairing workflow (initiate + accept dialogs). Fixed non-interactive UI: all subprocess-calling Tauri commands converted to async (were blocking main/webview thread), improved IPC detection, added WebKitGTK compositing workaround for Wayland, added error handling to all stores.
- 2026-04-01: Items 1-8 complete. Full two-panel UI with interactive Sunshine controls (monitor selector, codec/fps/bitrate), Moonlight controls (resolution picker, windowed/fullscreen, app selector, node selection), and mesh bar. Sunshine config read/write via sunshine.conf.
- 2026-03-30: Resolved all open questions. Tauri v2 + Rust + React. No daemon, no CLI. Side-by-side layout. Full mesh + personal status. v2 scaffold built.
- 2026-04-13: Architecture pivot — user requested standalone desktop GUI replacing CLI/daemon/TUI.
- 2026-03-26: Initial plan created from user feedback (now v1).
