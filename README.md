# orrbeam

A bidirectional Sunshine/Moonlight mesh — every machine is both host and client, all of them managed from a single Tauri desktop app.

## What it is

Sunshine streams a desktop. Moonlight receives it. Traditionally that's a one-way pair: machine A hosts, machine B watches, and you wire each direction up by hand on each box.

orrbeam collapses both sides into one application. Every node runs Sunshine *and* Moonlight under the same UI. Click any peer to connect to it; see the peers connected to you in the same panel. The mesh discovers itself over LAN mDNS or through your existing Headscale/orrtellite tailnet, and the trust between nodes is managed by signed Ed25519 identities — not Sunshine's PIN-pairing flow alone.

The split-pane UI is the whole product:

- **Left panel — Sunshine (Host).** Status, encoder, monitor, current resolution/FPS, list of clients streaming from you, start/stop.
- **Right panel — Moonlight (Client).** Discovered peers, connection state, "Connect (remote)" against any trusted node, inflight session.
- **Bottom bar — Mesh.** Every node, online/offline, trust state.

Connecting to a remote machine is a single click on a trusted peer. The control-plane handshake (start Sunshine on the other end, submit the pairing PIN, launch Moonlight here) happens behind the button.

## Why

> "I need a desktop UI that manages sunshine and moonlight in one two-parted standalone application."

Sunshine and Moonlight are excellent on their own. Treating them as two separate tools — each with its own config UI, paired one direction at a time — is the part that doesn't scale once you have more than two machines.

A few principles drive the design:

- **One mesh, not N pairs.** When you have a desktop, a laptop, a Mac, and a tablet, you don't want six pairing dialogs. You want one app showing all of them.
- **Self-hosted control plane.** Discovery rides on whatever mesh you already run — orrtellite (Headscale) by default, mDNS on plain LAN, static entries when you want explicit. No vendor cloud sits between your machines.
- **Identity, not PINs.** Each node has an Ed25519 signing key and a self-signed TLS cert pinned by trusted peers. Pairing happens once, out of band; from then on, "Connect (remote)" is one click.
- **Future use case: shared local play.** The longer-term target is using the same control plane to let a remote friend share input on a host's running emulator — one game, two players, two physical machines. The bidirectional control plane is the substrate for that; the input multiplexing layer is the next piece.

It is free, MIT-licensed, and explicitly aimed at being usable by people who didn't write it.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                  orrbeam app (Tauri v2 window)                   │
│                                                                   │
│  ┌─────────────────────┐         ┌─────────────────────┐         │
│  │  Sunshine panel     │         │  Moonlight panel    │         │
│  │  (host)             │         │  (client)           │         │
│  │  start / stop /     │         │  peer list /        │         │
│  │  connected clients  │         │  connect (remote)   │         │
│  └──────────┬──────────┘         └──────────┬──────────┘         │
│             │                                 │                   │
│             ▼                                 ▼                   │
│        ┌─────────────────────────────────────────────┐           │
│        │   Tauri IPC commands  (src-tauri/commands)  │           │
│        └────────────┬────────────────────┬───────────┘           │
│                     │                    │                        │
│           ┌─────────▼────┐    ┌──────────▼─────────┐             │
│           │ orrbeam-     │    │ orrbeam-net        │             │
│           │ platform     │    │ discovery + control│             │
│           │ (linux/mac/  │    │ plane client/server│             │
│           │  windows)    │    │                    │             │
│           └─────────┬────┘    └──────────┬─────────┘             │
│                     │                    │                        │
│                     ▼                    ▼                        │
│        Sunshine / Moonlight       mDNS + Headscale API           │
│        (subprocess)               + signed HTTPS (Ed25519)        │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────┐
              │      Other orrbeam nodes    │
              │  (orrion, orrpheus, …)      │
              └─────────────────────────────┘
```

| Component | Purpose |
|---|---|
| `src-tauri/` | Tauri v2 shell. Holds `AppState` (config, identity, node registry, platform, TLS, trusted peers). Defines IPC commands for `sunshine`, `moonlight`, `discovery`, `platform`, `settings`. |
| `crates/orrbeam-core` | Pure types. Config (YAML on disk), Ed25519 identity, `Node`/`NodeRegistry`, peer models, TLS cert helpers, wire-protocol primitives. |
| `crates/orrbeam-net` | Discovery (mDNS over `_orrbeam._tcp`, Headscale polling) and the control-plane HTTPS server/client. Signed-request canonical-string handling, nonce + timestamp checking. |
| `crates/orrbeam-platform` | Per-OS subprocess management for Sunshine and Moonlight. `linux.rs`, `macos.rs`, `windows.rs` implementing a shared `Platform` trait, picked at compile time via `#[cfg(target_os)]`. |
| `frontend/` | React 19 + TypeScript + Zustand + Tailwind. Lazy-loads the Tauri API at runtime so the same bundle can be opened in a plain browser with mock data for fast UI iteration. |
| `tests/e2e/` | Workspace member for end-to-end tests against the assembled control plane. |

The control plane (port `47782`, "orrbeam/1") is what makes one-click remote connect possible: it lets the local app reach into a trusted peer to start Sunshine, post the pairing PIN, and report status. Every signed request carries `X-Orrbeam-Timestamp`, `X-Orrbeam-Nonce`, and `X-Orrbeam-Signature` headers, with the canonical string `METHOD\nPATH\nTIMESTAMP\nNONCE\nSHA256(body)` — full normative description in [`docs/architecture.md`](docs/architecture.md) and [`docs/verifying_control_plane.md`](docs/verifying_control_plane.md).

## Quickstart

```bash
git clone git@github.com:TruStoryHnsl/orrbeam.git
cd orrbeam

# Frontend deps
cd frontend && npm install && cd ..

# Dev mode (Rust backend + Vite frontend with hot reload)
cargo tauri dev

# Production build
cargo tauri build
```

Useful local commands:

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check                      # license + advisory enforcement
cd frontend && npm run test
```

Frontend-only iteration (no Rust rebuild): `cd frontend && npm run dev` and open `http://localhost:1421/`. The IPC layer detects the missing Tauri runtime and serves mock data so panels render and exercise.

### Prerequisites

- Rust 1.80+ (rustup default toolchain)
- Node.js 20+ and npm
- Tauri v2 system dependencies for your OS (see [tauri.app](https://tauri.app/start/prerequisites/))
- Sunshine installed on machines that will host
- Moonlight installed on machines that will initiate sessions
- A working GPU encode path on hosts (NVENC, VAAPI, VideoToolbox, AMF, QuickSync — whichever Sunshine supports on your hardware)

### Runtime config

Per-user config directory (`~/.config/orrbeam/` on Linux, `~/Library/Application Support/orrbeam/` on macOS, `%APPDATA%\orrbeam\` on Windows) holds:

- `config.yaml` — node name, discovery toggles, static peers, Sunshine/Moonlight binary paths
- `trusted_peers.yaml` — managed by the Settings → Peers tab; not hand-edited

Identity material lives separately under `~/.local/share/orrbeam/`:

- `identity/signing.key` — Ed25519 signing key
- `tls/` — self-signed TLS cert derived from the signing key

## Features

- Two-pane host + client UI, both visible at once on every node
- Bidirectional mesh — any node can host or connect; roles are dynamic
- One-click "Connect (remote)" against trusted peers, including remote Sunshine start + PIN submit
- Mutual-trust handshake via TOFU dialog (no out-of-band PIN required for trust establishment)
- mDNS LAN discovery (`_orrbeam._tcp`) with manual fallback
- orrtellite/Headscale tailnet discovery via API key
- Static peer entries in `config.yaml` for explicit lists
- Ed25519-signed control-plane requests with nonce + timestamp replay protection
- TLS-1.3 with cert pinning between trusted peers
- Cross-platform Rust workspace (Linux + macOS + Windows active; mobile clients planned)
- Frontend mock mode for UI iteration without rebuilding the Tauri shell
- License enforcement on every dep (`cargo deny`, `scripts/check-licenses.sh`) — no GPL/LGPL/AGPL anywhere in the tree
- Rolling JSON log file in release builds — Linux: `~/.local/state/orrbeam/logs/orrbeam.log`, macOS: `~/Library/Application Support/orrbeam/logs/orrbeam.log`, Windows: `%LOCALAPPDATA%\orrbeam\logs\orrbeam.log`. Pretty tracing logs in dev with `RUST_LOG=orrbeam=debug`

## Status

**Active development. Single-author, single-mesh deployment today.** Day-to-day use is between `orrion` (CachyOS, RTX 3070), `orrpheus` (macOS M1 Pro), and `win11` (Windows 11). The Tauri shell, mesh discovery, signed control plane, and trust UI are working end-to-end on all three. Windows joins the mesh as a first-class node — both Sunshine host (NVENC / AMF / QuickSync) and moonlight-qt client.

| Target | OS | Host (Sunshine) | Client (Moonlight) | State |
|---|---|---|---|---|
| orrion | CachyOS Linux | NVENC (RTX 3070) | moonlight-qt | Primary dev |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app | Primary dev |
| win11 | Windows 11 | NVENC / AMF / QuickSync | Moonlight | Active |
| iPad / iPhone | iOS | n/a (client only) | Moonlight (Tauri mobile) | Planned |
| Android | Android | n/a (client only) | Moonlight (Tauri mobile) | Planned |

**Not yet supported / explicitly out of scope right now:**

- No headless / server mode — orrbeam is a desktop app, period. (The control plane runs in-process when the app is open.)
- No CLI front end. The GUI is the only interface.
- No vendor cloud; nothing phones home. If your tailnet/LAN is down, discovery degrades to static peers.
- Couch-co-op input multiplexing (one host, two players sharing a running emulator) is a planned use case, not a shipped feature.
- Mobile builds depend on Tauri 2's mobile pipeline maturing for our use case.

If you're trying it on a third machine and something doesn't work, please open an issue — that's exactly the feedback this stage of the project needs.

## Related projects

- **[orrtellite](https://github.com/TruStoryHnsl/orrtellite)** — self-hosted Headscale + WireGuard mesh. orrbeam uses its API as one of three discovery sources.
- **[orrchestrator](https://github.com/TruStoryHnsl/orrchestrator)** — the AI development hypervisor that drives most of this ecosystem's planning + parallel-session execution.
- **[concord](https://github.com/TruStoryHnsl/concord)** — the matching self-hosted chat/voice stack. Same "no vendor cloud in the middle" posture as orrbeam.
- **Sunshine** ([upstream](https://github.com/LizardByte/Sunshine)) — the host streamer orrbeam wraps.
- **Moonlight** ([upstream](https://github.com/moonlight-stream)) — the client orrbeam launches and configures.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Conventional commits, feature branches, license-clean dependencies, no GPL.

## License

[MIT](LICENSE).
