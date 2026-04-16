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
10. [x] **System tray** — Minimize to tray, quick connect shortcuts
11. [ ] **Mobile builds** — iOS + Android client-only mode
12. [ ] **Windows support** — Build + test on Windows
13. [x] **Persistent multi-node registry** (INS-001) — Extend `crates/orrbeam-core/src/node.rs` `NodeRegistry` with YAML-backed persistence in `~/.config/orrbeam/known_nodes.yaml`. Add `last_seen: Option<DateTime<Utc>>` field to `Node`. Expose `add`/`remove`/`list` Tauri commands (likely in `src-tauri/src/commands/discovery.rs`). Registry is currently ephemeral (in-memory `HashMap`); discovery repopulates it each run via mDNS/orrtellite. This item makes the known-peer list durable across restarts, independent of whether the peer is currently reachable. Offline-but-known peers should still render in the Moonlight node browser with a greyed-out status. **Acceptance**: add a node manually → restart app → node still listed with correct last-seen timestamp. Independent of the shared-control work below.

### Phase 3 — Shared-control co-op (design spike + implementation)

Shared-control introduces a fundamentally new session mode: 1:N input multiplexing where the host's local user and one or more remote participants all drive the same running process in real time. Primary target is emulator couch co-op. This is **not** a config tweak on the existing 1:1 Sunshine/Moonlight pipeline — it's a new crate/module, a new session type, and new platform-specific input plumbing. Design decisions gate implementation.

**Dependency graph:**
```
INS-003 (input arbitration)  ──┐
INS-004 (capacity benchmark) ──┤──▶ INS-002 (shared-control impl)
INS-005 (bridge relay)       ──┘
```
INS-002 is blocked until INS-003 resolves at a minimum. INS-004 sets the default/hard cap. INS-005 is optional but, if adopted, reshapes the network topology significantly (see Open Conflicts).

14. [x] **Design spike: multi-input arbitration** (INS-003) — status: `design`. Decide how the host OS distinguishes and routes multiple simultaneous keyboards/mice/gamepads from host + remote participants. Research options: per-participant virtual input device (uinput on Linux, IOHIDUserDevice on macOS, Interception/ViGEm on Windows), emulator player-slot routing, conflict resolution when two participants press the same key, and whether the host's local user continues to use real input devices unchanged. Deliverable: `docs/design/shared-control-input.md` with chosen approach per platform. **Acceptance**: documented design that lets host + two remote participants each drive a distinct emulator player slot without input collisions, reviewed before implementation begins.

15. [x] **Design spike: concurrent participant capacity** (INS-004) — status: `design`. Benchmark a single host (orrion as primary test rig) under shared-control load as participant count scales from 1 → N. Measure: CPU %, GPU encode queue depth, outbound bandwidth, per-participant input→display latency. Deliverable: `docs/design/shared-control-capacity.md` with measured curve, recommended default cap, hard limit for `config.yaml`. **Acceptance**: documented capacity envelope + config knob names. Prerequisite: INS-002 prototype or equivalent test harness exists to generate load.

16. [x] **Design spike: server-bridge relay feasibility** (INS-005) — status: `design`. Investigate using a user-owned server (candidate: orrgate) as a relay/bridge to fan out encoded video and aggregate input, offloading the host and raising the concurrent participant ceiling beyond direct host-to-participant mode. Evaluate: SFU-style video fan-out, input aggregation on the bridge, added latency tax, bandwidth topology `host→bridge→participants` vs. mesh, TLS/identity model. Deliverable: `docs/design/shared-control-bridge.md` with feasibility verdict + crossover point where bridge mode beats direct mode, plus prototype if viable. **Conflict**: this contradicts the "No headless mode / orrgate not supported" decision in the Resolved Decisions table. See Open Conflicts.

17. [ ] **Shared-control session mode implementation** (INS-002) — status: `scaffold-impl-partial`, **blocked by INS-003 (hard), informed by INS-004 and INS-005**. Scaffold landed (`crates/orrbeam-platform/src/shared_control.rs`): `SharedControlSession` trait + `LinuxSharedControlSession` stub with `uinput` fd slots; macOS/Windows stubs return `Unsupported`; `Config` extended with `shared_control_enabled`/`max_participants`. Full impl still blocked. Implement a new session type in `crates/orrbeam-platform/` (likely a new `shared_control` module, or a sibling `orrbeam-coop` crate if the surface grows large) that lets a host permit one or more remote participants to control it in parallel with the host's local user, multiplexing all input streams into the host OS in real time. The existing 1:1 Sunshine/Moonlight pipeline handles video out; the new work is input demux on the participant side and input remux on the host side, plus a session-mode toggle in the Sunshine panel ("Solo" vs. "Shared-control"). **Acceptance**: host + 2 remote participants each drive a distinct emulator player slot in a real emulator (RetroArch or PCSX2) with no input collisions, sustained for 10 min at the default cap from INS-004.

### Phase 4 — Commercial-grade OSS readiness (governance, testing, CI, docs)

Orrbeam's scope moves from `public` → `commercial`. The user-facing promise is unchanged (free, OSS-friendly) but the engineering rigor bar moves from "public" profile to "commercial" profile per `~/.claude/CLAUDE.md` — comprehensive tests, full docs, CI/CD gates, license compliance, production-grade error handling, and structured logging. This phase is orthogonal to Phase 3 and can be worked in parallel.

**Dependency graph:**
```
OPT-001 (scope flip)  ──▶ governs all Phase 4 rigor
OPT-006 unit     ─┐
OPT-007 integ    ─┼──▶ OPT-009 (CI) ──▶ OPT-010 (license scan in CI)
OPT-008 e2e      ─┘                  ──▶ OPT-015 (API ref publish)
OPT-017 (lint/fmt cfg) ──▶ OPT-009 (CI enforces)
OPT-020 (repo verify)  ──▶ OPT-009 (GH Actions needs remote)
```

#### Governance & licensing

18. [x] **Set project scope to commercial** (OPT-001) — Done 2026-04-09. `.scope` now contains `commercial`. Gates every other Phase 4 item and the Phase 5 Control Plane work: all rigor (testing, docs, CI, license compliance) follows the commercial profile from `~/.claude/CLAUDE.md`.
19. [x] **Add LICENSE file** (OPT-002) — Add MIT `LICENSE` at repo root (matches `license = "MIT"` already declared in `Cargo.toml` workspace.package). Include copyright line `Copyright (c) 2026 Colton Orr`. Verify the frontend `package.json` and any other manifests reference the same license string.
20. [x] **Add CODE_OF_CONDUCT.md** (OPT-005) — Drop the Contributor Covenant v2.1 template at `orrbeam/CODE_OF_CONDUCT.md`. Replace the `[INSERT CONTACT METHOD]` placeholder with a real maintainer contact (email or GitHub username). No other edits.
21. [x] **Add SECURITY.md** (OPT-018) — Create `orrbeam/SECURITY.md` documenting: supported versions (initially `0.x` only), private disclosure channel (GitHub Security Advisories on `TruStoryHnsl/orrbeam`, plus fallback email), expected triage SLA (e.g. 72h ack / 14d initial assessment), and scope (in-scope: core crates, Tauri IPC, network discovery; out-of-scope: Sunshine/Moonlight upstream bugs).
22. [x] **Verify independent repo + GitHub remote** (OPT-020) — Confirm `orrbeam/.git` exists and `origin` points to `git@github.com:TruStoryHnsl/orrbeam.git` (public visibility). If missing, run `/repo-init orrbeam --visibility public`. Audit `.gitignore` covers: `target/`, `node_modules/`, `dist/`, `.env`, `.env.*`, `*.log`, editor dirs (`.vscode/`, `.idea/`), and Tauri build artifacts (`src-tauri/target/`, `src-tauri/gen/`). This item is a hard prerequisite for OPT-009 (GitHub Actions needs the remote).

#### Documentation

23. [x] **Write comprehensive README.md** (OPT-003) — Rewrite `orrbeam/README.md` to cover, in order: (a) one-paragraph pitch — "bidirectional Sunshine/Moonlight mesh, every node is both host and client"; (b) feature highlights; (c) supported platforms table (pulled from PLAN.md "Target Platforms"); (d) prerequisites (Rust 1.80+, Node 20+, Sunshine + moonlight-qt installed on host/client respectively, platform-specific GPU encoder requirements); (e) quickstart build + run (`cargo tauri dev`); (f) configuration pointer (`~/.config/orrbeam/`); (g) architecture overview with a pointer to `docs/architecture.md`; (h) contribution pointer to `CONTRIBUTING.md`; (i) license line. Target audience: a Rust/TS dev who has never seen the repo.
24. [x] **Add CONTRIBUTING.md** (OPT-004) — Create `orrbeam/CONTRIBUTING.md` covering: local dev setup (`cargo tauri dev`, `cd frontend && npm install`), workspace layout recap (3 crates + `src-tauri` + `frontend`), branch naming (`feat/`, `fix/`, `refactor/`, `chore/`), conventional commit format with examples, pre-commit expectations (`cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `npm run test` in frontend), PR process (CI must pass, one review required), and how to run the API-doc publish step once OPT-015 lands.
25. [x] **Write architecture documentation** (OPT-014) — Create `orrbeam/docs/architecture.md` describing: crate boundaries (`orrbeam-core` = types/config/identity, `orrbeam-net` = discovery, `orrbeam-platform` = OS abstraction + Sunshine/Moonlight process management, `src-tauri` = IPC commands + app glue, `frontend` = React UI), data flow from discovery → node registry → UI stores → Tauri IPC → platform layer, node topology (bidirectional mesh), protocol choices (Sunshine pairing, Moonlight streaming, mDNS `_orrbeam._tcp`, orrtellite Headscale API), and extension points for new platforms or session modes. Include at least one mermaid diagram showing the crate dependency graph and one ASCII diagram of the Tauri IPC surface.
26. [x] **Add API/module reference documentation** (OPT-015) — Wire `cargo doc --workspace --no-deps` into CI (OPT-009) and publish the output to GitHub Pages from the `main` branch on successful release-tagged builds. Gate all `pub` items in `orrbeam-core`, `orrbeam-net`, `orrbeam-platform` behind `#![warn(missing_docs)]` and backfill doc comments for the public surface. Document the publish step in `CONTRIBUTING.md` (OPT-024). **Depends on OPT-009** for the publish action.
27. [x] **Add issue and PR templates** (OPT-016) — Create `.github/ISSUE_TEMPLATE/bug_report.md` (fields: orrbeam version, OS + GPU, Sunshine/Moonlight versions, reproduction steps, logs), `.github/ISSUE_TEMPLATE/feature_request.md` (problem, proposed solution, alternatives considered), and `.github/pull_request_template.md` (summary, linked issue, test plan, checklist: conventional commit, tests added, docs updated, `cargo fmt` + `clippy` clean).

#### Testing

28. [x] **Build unit test suite** (OPT-006) — Add `#[cfg(test)] mod tests` blocks across `crates/orrbeam-core` (config parse/write, Ed25519 identity roundtrip, `NodeRegistry` add/remove/list, sunshine.conf read/write), `crates/orrbeam-net` (mDNS record construction/parsing, orrtellite Headscale response parsing with recorded fixtures), and `crates/orrbeam-platform` (OS/GPU detection with mocked platform probes, Sunshine/Moonlight install detection). Use `tempfile` for filesystem tests. Frontend unit tests via `vitest` (already wired in `frontend/package.json`) covering Zustand stores in `frontend/src/stores/`. Single command: `cargo test --workspace && (cd frontend && npm run test)` — document in README (OPT-023).
29. [x] **Build integration test suite** (OPT-007) — Add `crates/orrbeam-platform/tests/` integration tests exercising process-level Sunshine/Moonlight orchestration with a mock binary fixture (a trivial bash script that simulates Sunshine's start/stop behavior and writes sentinel files). Add `crates/orrbeam-net/tests/` covering end-to-end mDNS discovery against a loopback advertiser. Add `src-tauri/tests/` covering the IPC command surface in `src-tauri/src/commands/` with Tauri's mock IPC harness. Real-hardware tests (actual GPU encoding, actual LAN peers) gated behind a `#[ignore]` attribute with a doc comment on how to opt in.
30. [x] **Add end-to-end test scenarios** (OPT-008) — Define e2e flows in `orrbeam/tests/e2e/` (workspace-root integration crate): (1) node pairing — start Sunshine on node A, request PIN from node B, complete pairing; (2) stream start/stop — pair, start Moonlight stream, verify frames received, stop cleanly; (3) bidirectional handoff — A hosts B, then B hosts A in the same session. Document hardware prerequisites (two machines with GPU encoders, Sunshine + moonlight-qt installed) in `tests/e2e/README.md`. These are `#[ignore]` by default and run in a manual `e2e` CI job on self-hosted runners if/when available.

#### CI/CD & code quality

31. [x] **Add linter and formatter configuration** (OPT-017) — Commit `rustfmt.toml` (edition 2024, `max_width = 100`, `imports_granularity = "Crate"`), `clippy.toml` (workspace-wide lint allowlist), and enable `#![warn(clippy::pedantic)]` on new crates. Frontend: commit `.eslintrc.cjs` with `@typescript-eslint/recommended` + React 19 rules, and `.prettierrc` matching Tailwind's conventions. Add `npm run lint` and `npm run format:check` scripts to `frontend/package.json`. All four tools enforced by CI (OPT-030/-033). **Prereq for OPT-033** (CI cannot enforce what doesn't exist).
32. [x] **Set up semantic versioning and CHANGELOG** (OPT-013) — Initialize `orrbeam/CHANGELOG.md` following Keep a Changelog 1.1.0. Backfill the current state as `## [0.1.0] - 2026-04-23` with the v2 Tauri scaffold milestone (items 1-10 complete). Document in `CONTRIBUTING.md` that releases use `/release orrbeam <bump>` and the changelog is auto-generated from conventional commits. Align workspace version fields in `Cargo.toml` and `frontend/package.json` (both currently `0.1.0`).
33. [x] **Set up CI/CD pipeline** (OPT-009) — Add `.github/workflows/ci.yml` running on every push and PR: (a) `cargo fmt --all -- --check`; (b) `cargo clippy --workspace --all-targets -- -D warnings`; (c) `cargo test --workspace`; (d) `cargo build --workspace --release`; (e) frontend lint (`npm run lint`), format check, typecheck (`tsc -b`), unit tests (`npm run test`), build (`npm run build`); (f) full `cargo tauri build` on Linux + macOS + Windows matrix (Tauri action). Fail fast on any step. Cache `~/.cargo`, `target/`, and `frontend/node_modules`. **Depends on OPT-028/-029/-030 tests existing** and **OPT-031 lint configs existing** and **OPT-022 repo remote being live**.
34. [x] **Add dependency license audit** (OPT-010) — Add `cargo-deny` with `deny.toml` at repo root: advisory DB check, license allowlist (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unicode-DFS-2016, Zlib), `deny` on GPL/AGPL/LGPL, source registry pinned to crates.io. Add an `npm-license-checker` step (or `license-checker-rseidelsohn`) covering `frontend/node_modules/` with the same allowlist. Wire both into the CI workflow from OPT-033 as a dedicated `license-audit` job that fails the build on any disallowed license. **Depends on OPT-033** (CI must exist).

#### Production hardening

35. [x] **Add input validation and error handling audit** (OPT-011) — Audit every Tauri command in `src-tauri/src/commands/` (sunshine, moonlight, discovery, platform, settings) for: argument validation (reject malformed node IDs, out-of-range ports, non-UTF-8 paths), structured error types (extend or replace ad-hoc `anyhow` returns with `thiserror`-derived crate-level error enums in `orrbeam-core`), and user-facing messages (no raw `Debug` formatting of errors into the frontend — map to structured error codes the React layer can localize). Audit config file loading in `orrbeam-core` for schema validation on read. Deliverable: a single `OrrbeamError` type per crate plus a top-level `AppError` in `src-tauri` that serializes cleanly to the frontend.
36. [x] **Add structured logging** (OPT-012) — `tracing` and `tracing-subscriber` are already in `[workspace.dependencies]` but underused. Audit every crate and `src-tauri` for `println!`/`eprintln!`/`dbg!` calls and replace with `tracing::{info,warn,error,debug}` macros with structured fields (e.g. `tracing::info!(node_id = %id, "pairing complete")`). Configure `tracing-subscriber` in `src-tauri/src/lib.rs` `run()` with `EnvFilter` (respect `RUST_LOG`), JSON output in release builds, pretty output in debug builds. Log sink: stdout for `cargo tauri dev`, OS-appropriate log file (`~/.local/state/orrbeam/orrbeam.log` on Linux) for packaged builds with rotation via `tracing-appender`.
37. [x] **Audit and remove hardcoded values** (OPT-019) — Grep the workspace for hardcoded IPs (`192.168.`, `127.0.0.1` outside of test fixtures), hostnames (`orrion`, `orrpheus`, `orrgate` — these belong in config or discovery, not source), absolute paths (`/home/`, `/Users/`, `/mnt/`), and any embedded credentials or tokens. Move all to: (a) `orrbeam-core` config with documented defaults in `Config::default()`; (b) environment variables for ops-time overrides (document each in a new `orrbeam/.env.example`); (c) discovery layer for peer addresses (already the case — verify no leaks). Produce a short report of what was moved and to where.

## Resolved Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| GUI toolkit | Tauri v2 + Rust + React | Cross-platform (desktop + mobile), lightweight, matches concord v2 stack |
| Daemon | Eliminated | No headless use case — orrbeam is a desktop app. GUI manages processes directly. |
| CLI | Eliminated | GUI is the sole interface |
| Layout | Side-by-side | Left = Sunshine (host), Right = Moonlight (client) |
| Mesh visibility | Both views | Full mesh + personal hosting/connection status |
| Headless (orrgate) | Not supported | Orrgate is a services VM — SSH suffices, no graphical desktop needed. **Status: under review — see Open Conflicts (INS-005 bridge mode).** |
| Project scope | `commercial` (was `public`) | OPT-001 — orrbeam is now positioned as a commercial-grade free application; all subsequent rigor (testing, docs, CI, licensing) follows the commercial profile from `~/.claude/CLAUDE.md`. |

## Open Conflicts

- **Headless mode vs. server-bridge relay (INS-005 vs. Resolved Decisions row "Headless (orrgate)")**. The current design explicitly rules out headless/orrgate operation on the grounds that orrbeam is a desktop app with no server use case. INS-005 asks for exactly that: a server-hosted relay/bridge (likely on orrgate) that fans out video and aggregates input to raise the shared-control participant ceiling. A bridge IS a headless service — it runs on a box with no graphical desktop, encodes/forwards streams, and exposes an input-aggregation endpoint. **Resolution gating**: this conflict does not need to be resolved until INS-005's feasibility spike (item 16) returns a verdict. If the feasibility report concludes bridge mode is viable and worth building, the "no headless" decision must be revisited — likely by splitting orrbeam into `orrbeam-desktop` (the GUI, unchanged) and `orrbeam-bridge` (a new headless service crate/binary), so the desktop-only promise of the GUI app remains intact while the bridge ships as a separate artifact. If INS-005 concludes bridge mode is not worth the complexity, the "no headless" decision stands as-is and this conflict is closed.

## Recent Changes
- 2026-04-23: **Intake of `plans/2026-04-23-21-40.md` — 20 new instructions (OPT-001..OPT-020).** Orrbeam re-positioned as commercial-grade free OSS. New **Phase 4: Commercial-grade OSS readiness** added (roadmap items 18-37), grouped into governance/licensing (18-22), documentation (23-27), testing (28-30), CI/CD & code quality (31-34), and production hardening (35-37) clusters. Scope changed `public` → `commercial` in Resolved Decisions. The user-facing promise is unchanged (free, OSS-friendly) but the engineering rigor bar moves from "public" profile to "commercial" profile per `~/.claude/CLAUDE.md`. Documented intra-phase dependencies: OPT-001 gates all, OPT-017 + tests (28-30) gate CI (33), CI (33) gates license audit (34) and API-doc publish (26), repo verification (22) gates CI (33).
- 2026-04-23: **Intake of `plans/2026-04-23-21-11.md` — 5 new instructions.** INS-001 (persistent multi-node registry) appended as roadmap item 13, extending the existing in-memory `NodeRegistry` in `crates/orrbeam-core/src/node.rs` with on-disk persistence and a `last_seen` field. INS-002..005 captured as a new **Phase 3: Shared-control co-op** block (roadmap items 14-17): three design spikes (INS-003 input arbitration, INS-004 capacity benchmark, INS-005 bridge-relay feasibility) gate the implementation of INS-002 (shared-control session mode). INS-005 flagged in new **Open Conflicts** section — server-bridge relay contradicts the "no headless mode" decision and the conflict will be resolved by the feasibility spike.
- 2026-04-04: **Item 10 done — System tray.** Tray icon with dynamic menu (live Sunshine/Moonlight status, online node quick-connect shortcuts), minimize-to-tray on window close, 5s background refresh, one-time frontend notification toast.
- 2026-04-01: **Item 9 done + interactivity bugfix.** Pairing workflow (initiate + accept dialogs). Fixed non-interactive UI: all subprocess-calling Tauri commands converted to async (were blocking main/webview thread), improved IPC detection, added WebKitGTK compositing workaround for Wayland, added error handling to all stores.
- 2026-04-01: Items 1-8 complete. Full two-panel UI with interactive Sunshine controls (monitor selector, codec/fps/bitrate), Moonlight controls (resolution picker, windowed/fullscreen, app selector, node selection), and mesh bar. Sunshine config read/write via sunshine.conf.
- 2026-03-30: Resolved all open questions. Tauri v2 + Rust + React. No daemon, no CLI. Side-by-side layout. Full mesh + personal status. v2 scaffold built.
- 2026-04-13: Architecture pivot — user requested standalone desktop GUI replacing CLI/daemon/TUI.
- 2026-03-26: Initial plan created from user feedback (now v1).

---

# Trusted-Peer Control Plane (2026-04-09)

## Goal

Build the orrbeam v2 **control plane**: a TLS-1.3 + Ed25519-signed HTTPS API on port `47782` that lets one node remotely start Sunshine, submit a pairing PIN, and initiate a local Moonlight stream against a trusted peer — so that clicking "Connect (remote)" from orrion against orrpheus requires **no human on the orrpheus side**. The authoritative design (22 sections, wire protocol, module map, state machine, security checklist) lives in `/home/corr/.claude/plans/wild-crunching-flute.md` — treat it as the single source of truth for every implementation decision.

## Status

`Status: queued — 18 work items, 0 completed, 0 in_progress`

## Reference

**Authoritative plan:** `/home/corr/.claude/plans/wild-crunching-flute.md`

Workers MUST read the relevant sections of that file in full before touching code. Do not invent design choices that aren't specified there; if something is unclear, raise it as an Open Question in `PLAN.md` rather than improvising.

Key section pointers:
- §5 — dependency wiring
- §7 — **wire protocol (normative — byte-exact)**
- §8 — TLS identity layer
- §9 — trusted peer storage
- §10 — control server (axum, router, middleware, nonce cache)
- §11 — control client (pinned + bootstrap modes, PinnedVerifier)
- §12 — `connect_to_peer` orchestration state machine
- §13 — mutual-trust flow end-to-end
- §14 — mDNS publishing
- §15 — UI layer (Settings drawer, Peers tab, TOFU, progress modal)
- §16 — **Work Item breakdown** (the 18 WIs chunked below)
- §17 — critical files list
- §18 — end-to-end verification runbook (orrion ↔ orrpheus)
- §19 — **security checklist (WI-18 gate)**

## Critical Constraints

> **Read before writing any code.** These are the traps most likely to bite you.

- **orrpheus SSH user is `coltonorr`, not `corr`.** Every doc, example, and runbook step that mentions orrpheus must use `coltonorr@orrpheus`. The project home on orrpheus is `/Users/coltonorr/projects/orrbeam`.
- **Scope is `public`.** Add docstrings on new public APIs (TLS, peers, wire, server, client, Tauri commands). Do not hardcode IPs, paths, or credentials — use env vars or config. If you touch README/LICENSE expectations and something is missing, flag it once. No raw stack traces in user-facing error messages.
- **No CLI surface. HTTP path only.** Do NOT add `--headless-accept-pin`, `orrbeam-cli`, or any detached mode. The HTTP `POST /v1/pair/accept` endpoint is the sole mechanism. This was asked about explicitly and the answer was "no".
- **No `/v1/loop` or `/v1/connect-back` endpoints.** Bidirectional capability means each direction is user-initiated, not auto-looped. These paths from the old v1 Python daemon are explicitly **out of scope**.
- **Port 47782 collides with the old v1 Python daemon.** Some dev machines may still have it running. WI-6 / WI-9 must produce a clear error on bind failure, and the runbook in WI-16 must tell users to kill the old daemon first (`lsof -i :47782`, `pkill -f 'orrbeam.*daemon'`).
- **No plaintext HTTP on 47782 — ever.** TLS 1.3 only, even on loopback during bootstrap.
- **Bind scope is `0.0.0.0:47782`** — not loopback. Mesh reachability is required from day one.
- **Two client modes must not mix.** `danger_accept_invalid_certs(true)` may only appear in `bootstrap_hello`, `send_mutual_trust_request`, `poll_mutual_trust_request`. Pinned client requests NEVER use it. WI-18 greps for this.
- **Never log the PIN, signing key bytes, Sunshine password, or cert PEM.** Custom `Debug` on `ControlState` / `Identity` masks them.
- **Verification uses `VerifyingKey::verify_strict`**, never `.verify` (which accepts malleable sigs).
- **Hash the request body *as received*** — no `serde_json::to_vec(&parsed)` round-trip. Body bytes are buffered with a 64 KiB limit via `axum::body::to_bytes`.

## Work Items

All 18 WIs. Complexity is S/M/L. Dependencies reference other WI IDs. Claim a row by changing Status from `pending` to `in_progress <session-id>`. Mark `completed` only after the WI's verify step passes. WI-6 is split into two phases (scaffold + unauthenticated routes, then authenticated handlers) — treat them as one WI for counting, two commits for execution.

| ID | Description | Complexity | Deps | Lane | Status |
|----|-------------|-----------:|------|:----:|--------|
| WI-1  | Workspace dependency wiring (axum, axum-server, rustls, rcgen, sha2, base64, time, uuid, tokio-util) per §5 | S | — | A | **done 2026-04-09** |
| WI-2  | `orrbeam-core::tls` — `TlsIdentity::load_or_create` + `rustls_server_config` per §8 | M | WI-1 | A | **done 2026-04-09** (4 tests) |
| WI-3  | `orrbeam-core::peers` — `TrustedPeer`, `PeerPermissions`, YAML store per §9 | M | WI-1 | B | **done 2026-04-09** (7 tests) |
| WI-4  | `orrbeam-core::wire` — canonical signing, sign/verify helpers, header consts, `build_hello_payload` per §7 | M | WI-1 | B | **done 2026-04-09** (11 tests) |
| WI-5  | `orrbeam-net::server::nonce` — per-key-id replay cache + GC task per §10.6 | S | WI-1 | B | **done 2026-04-09** (6 tests) |
| WI-6  | Control server: scaffold + unauthenticated routes (`/v1/hello`, mutual-trust request/poll, rate limits), plus authenticated handlers (`status`, `sunshine/start`, `sunshine/stop`, `pair/accept`, `peers`) with `require_signed` middleware — split commits allowed (WI-6a scaffold, WI-6b auth handlers) | L | WI-2, WI-3, WI-4, WI-5 | A | **done 2026-04-09** (21 tests) |
| WI-7  | `orrbeam-net::client` — `ControlClient`, `PinnedVerifier`, sign helper, pinned + bootstrap modes (§11) | L | WI-2, WI-4, WI-5 | B | **done 2026-04-09** (14 tests + 1 doctest) |
| WI-8  | `orrbeam-net::mdns::register` + `Node.cert_sha256` field + `DiscoveryManager::start` TLS param (§14) | M | WI-2 | C | **done 2026-04-09** |
| WI-9  | Platform `Send + Sync` audit + `AppState` wiring: spawn control server, `TauriEventEmitter`, shutdown token (§10.1, §17) | M | WI-6, WI-3 | A | **done 2026-04-15** |
| WI-10 | `src-tauri::commands::remote` — 11 commands incl. `connect_to_peer` state machine + mutual-trust flow (§12, §13) | L | WI-7, WI-9 | B | **done 2026-04-15** |
| WI-11 | `get_tls_fingerprint` command in `src-tauri::commands::settings` | S | WI-2 | C | **done 2026-04-15** |
| WI-12 | `frontend/stores/peers.ts` + `types/peers.ts` + mocks in `api/tauri.ts` (§15.7) | M | WI-10 signatures stable | D | **done 2026-04-15** |
| WI-13 | Settings drawer skeleton — `SettingsDrawer`, `GeneralTab`, `AboutTab`, gear button in `Shell.tsx` (§15.1) | M | WI-11, WI-12 | D | **done 2026-04-15** |
| WI-14 | Peers tab + `TofuDialog` + `MutualTrustRequestDialog` + `MutualTrustPendingModal` (§15.2–15.4) | L | WI-13 | D | **done 2026-04-15** |
| WI-15 | `PeeringProgressModal` + `NodeCard` "Connect (remote)" button + `onPeeringProgress` helper (§15.5–15.6) | M | WI-10, WI-14 | D | **done 2026-04-15** |
| WI-16 | Documentation: update `CLAUDE.md` with control-plane section + port table + `coltonorr@orrpheus` runbook example; `frontend/README.md` settings drawer notes | S | — (independent track) | C | **done 2026-04-15** |
| WI-17 | End-to-end orrion ↔ orrpheus smoke test + `docs/verifying_control_plane.md` runbook (§18) | M | WI-10, WI-15 | A | pending |
| WI-18 | Security review pass per §19 checklist; dispatch `feature-dev:code-reviewer` agent; blocks merge | M | all above | — | pending |

**Note on WI-6 split:** if a worker takes WI-6, they may land the scaffold + unauth handlers as one commit (`WI-6a`) and the signed/authenticated handlers as a second commit (`WI-6b`). Both must be complete before WI-9 or WI-7 can finish their integration tests, but WI-7 can start in parallel against a stub server. Per §16.2, WI-6b is Worker A's responsibility.

## Dependency Graph

```
WI-1
 ├─ WI-2 ─┬─────────────────────┐
 │        │                     │
 ├─ WI-3 ─┤                     │
 │        │                     │
 └─ WI-4 ─┼─ WI-5 ──┐            │
          │         │            │
          │         ├─ WI-8 ─────┤   (parallel with 6/7)
          │         │            │
          │         └─ WI-6 ─ WI-6b ─┐
          │                          ├── WI-9 ── WI-10 ─┐
          │                  WI-7 ───┘                  │
          │                                             │
          └─ WI-11 ─────┐                               │
                        │                               │
       WI-10 sigs ── WI-12 ── WI-13 ── WI-14 ── WI-15 ──┤
                                                        │
                                               WI-17, WI-18
       WI-16 is independent (doc track — any worker, anytime)
```

Plain-bullet form for clarity:

- **WI-1** blocks everything (dep wiring).
- **WI-2** blocks WI-6, WI-7, WI-8, WI-11 (TLS identity is needed by server, client, mDNS, About tab).
- **WI-3** blocks WI-6, WI-9 (peer store needed by server + AppState).
- **WI-4** blocks WI-6, WI-7 (wire protocol shared by server and client).
- **WI-5** blocks WI-6, WI-7 (nonce cache shared by both).
- **WI-6** (scaffold + auth handlers a.k.a. 6b) blocks WI-9.
- **WI-7** blocks WI-10.
- **WI-8** blocks nothing critical but is required for mDNS-discovered peers to carry `cert_sha256`.
- **WI-9** blocks WI-10, WI-17.
- **WI-10** blocks WI-12 (signatures), WI-15, WI-17.
- **WI-11** blocks WI-13 (AboutTab needs the fingerprint command).
- **WI-12** blocks WI-13.
- **WI-13** blocks WI-14.
- **WI-14** blocks WI-15.
- **WI-15** blocks WI-17.
- **WI-16** has no deps — pick up any time.
- **WI-17** blocks WI-18.
- **WI-18** gates merge.

## Parallel Assignment (§16.2)

Four workers can progress simultaneously once WI-1 lands:

- **Worker A — critical path (Rust server/integration):** WI-1 → WI-2 → WI-6 (scaffold + 6b) → WI-9 → WI-17.
- **Worker B — network-client lane (Rust client + types):** WI-3 (after WI-1) → WI-4 → WI-5 → WI-7 → WI-10.
- **Worker C — discovery + docs:** WI-8 (after WI-2) → WI-11 (after WI-2) → WI-16 (independent; can start immediately).
- **Worker D — frontend lane:** waits for WI-10 signatures to be stable, then WI-12 → WI-13 → WI-14 → WI-15.
- **Final gate:** WI-18 (security review) runs after all of the above are complete. Use `feature-dev:code-reviewer` agent.

Workers in different lanes should avoid touching each other's files. Cross-lane changes (e.g. Worker D needing a new field on a Tauri command) go through the primary owner of that file via `PLAN.md`.

## Definition of Done

Merge is allowed only when **every** box below is true. Derived from §18 verification plan + §19 security checklist.

### Functional end-to-end (§18)

- [ ] `ss -tlnp | grep 47782` (Linux) / `lsof -i :47782` (macOS) shows orrbeam bound on `0.0.0.0:47782` on both orrion and orrpheus.
- [ ] `curl --insecure https://<peer>:47782/v1/hello | jq .` returns `{node_name, ed25519_fingerprint, cert_sha256, control_port: 47782, ...}` on both hosts.
- [ ] `cargo run -p orrbeam-net --example control_curl -- --peer <ip> --path /v1/status` returns `401 unknown_key` before trust, `200` after mutual trust.
- [ ] Mutual-trust flow: orrion clicks "Request mutual trust", orrpheus sees the `MutualTrustRequestDialog`, clicks Approve, both peer lists show the other peer within seconds — no hand-editing of YAML.
- [ ] `Connect (remote)` on the orrion NodeCard drives `PeeringProgressModal` through `resolving → probing → remote_starting → pin_generating → paired_parallel → streaming_local → done`. Orrpheus's Sunshine is observed to start; moonlight-qt on orrion begins streaming the orrpheus desktop.

### Failure-path coverage (§18.6)

- [ ] Removing orrion from orrpheus's trusted peers causes the next connect attempt to fail with `unknown_key` visible in the progress modal.
- [ ] Tampering `cert_sha256` in orrion's `trusted_peers.yaml` for orrpheus produces a `cert pin mismatch` TLS error.
- [ ] Replaying a signed request within 5 minutes returns `replay`.
- [ ] 60 s clock skew returns `clock_skew`.
- [ ] Stopping orrbeam on orrpheus produces a hard error "peer unreachable — is orrbeam running on orrpheus?" with a Retry button. No silent fallback.

### Security checklist (§19 — WI-18 gate)

- [ ] §19.1 **Signature correctness**: canonical string identical in `wire::sign` and `wire::verify`; server hashes **raw** body bytes (no serde round-trip); `verify_strict`, not `verify`; empty signature header rejected explicitly.
- [ ] §19.2 **Replay protection**: every authenticated route goes through `require_signed`; nonce cache is per-key-id; GC task spawned; eviction tested.
- [ ] §19.3 **TLS pinning**: `danger_accept_invalid_certs(true)` appears ONLY in `bootstrap_hello`, `send_mutual_trust_request`, `poll_mutual_trust_request` (grep-verified); `PinnedVerifier` uses constant-time compare; rustls client restricted to TLS 1.3; name verification explicitly skipped with a code comment.
- [ ] §19.4 **Permission coverage**: every authenticated handler checks the matching `peer.permissions.*` bit; unauth handlers never read `PeerContext`; `trusted_full()` covers every perm.
- [ ] §19.5 **No secrets in logs**: PIN redacted; signing key bytes never reach `tracing`; sunshine password never reaches `tracing`; `Debug` masks key material on `ControlState` and `Identity`.
- [ ] §19.6 **No panic paths**: `cargo clippy -p orrbeam-net -- -D clippy::unwrap_used -D clippy::expect_used` passes; axum body buffering uses `to_bytes(body, 64 * 1024)`.
- [ ] §19.7 **File permissions**: `trusted_peers.yaml`, `control.key.pem`, `signing.key` all `0o600` on Unix; parent dirs pre-created.
- [ ] §19.8 **Concurrency**: no `.blocking_lock()` on tokio RwLocks in request path; middleware takes `.read()` on peer store; handlers `.write()` only for `touch_last_seen`.
- [ ] §19.9 **Mutual-trust anti-abuse**: `/v1/mutual-trust-request` rate-limited to 3/min/IP and max 1 pending globally; 60 s expiry enforced; approval requires explicit user click.
- [ ] §19.10 **Plan compliance**: no `/v1/loop` or `/v1/connect-back`; no CLI surface added; no plaintext 47782 listener; mDNS TXT exposes only fingerprint and `cert_sha256`, never private keys or cert PEM.

### Public-scope hygiene

- [ ] New public APIs (TLS, peers, wire, server, client, Tauri commands) have docstrings.
- [ ] No hardcoded IPs or paths — config or env vars only.
- [ ] `coltonorr@orrpheus` used consistently everywhere orrpheus is mentioned.
- [ ] `docs/verifying_control_plane.md` exists and the steps run clean on a fresh checkout.

## How to pick up work

1. **Claim:** open this file, find a WI whose `Status` is `pending` and whose dependencies are all `completed`. Change its row to `in_progress <your-session-id>` (e.g. `in_progress claude-20260409-a`) and commit PLAN.md alone in one conventional commit (`chore(plan): claim WI-7`).
2. **Read the spec:** open `/home/corr/.claude/plans/wild-crunching-flute.md` and read every section referenced by your WI's description. Do NOT skim. The wire protocol in §7 is byte-exact — matching it against the server is more important than the elegance of your abstractions. When in doubt re-read rather than guess.
3. **Implement + verify:** write the code, run the WI's `verify` step from §16. For Rust WIs, at minimum `cargo check --workspace` and the unit tests named in the WI must pass. For frontend WIs, mock mode must still load cleanly in a browser.
4. **Commit:** conventional commit per the scope rules in `~/projects/CLAUDE.md` (e.g. `feat(net): add orrbeam-net::server scaffold (WI-6)`). If a WI is split across multiple commits, reference the WI ID in every commit body.
5. **Close:** update this file — change `in_progress <id>` to `completed` and add a one-line note in the Recent Changes log at the bottom of this section (e.g. `- 2026-04-10: WI-2 done — TlsIdentity load_or_create + rustls_server_config, sha256 stable across reloads.`). Commit PLAN.md in a second chore commit.
6. **If blocked:** do NOT silently guess. Add an `Open Questions` block to `PLAN.md` under the Trusted-Peer Control Plane entry, change the WI's Status to `blocked <your-session-id>: see PLAN`, commit, and stop. The next session will resolve the question with the user and unblock.
7. **Cross-lane changes:** if your WI requires changing a file owned by another lane (e.g. Worker D needing a new field on a Tauri command that Worker B is responsible for), append a request to `PLAN.md` rather than editing the other lane's files directly — this prevents merge pain when commits land in parallel.
8. **WI-18 is a hard gate.** Do not merge any phase-D frontend changes to `main` until WI-18 has signed off on WI-2..WI-15. WI-16 and WI-17 can land on `main` continuously as docs/runbook.

## Recent Changes (this section)

- 2026-04-15: WI-9..WI-16 complete. Full control plane UI: PeersTab (identity, trusted peers list, inbound trust requests, add-peer form), TofuDialog, MutualTrustRequestDialog, MutualTrustPendingModal, PeeringProgressModal, NodeCard "Connect (remote)" button. WI-17 (E2E runbook) and WI-18 (security review) remain.
- 2026-04-09: Trusted-Peer Control Plane planning doc ingested. 18 WIs queued for 4-worker parallel execution. Authoritative plan: `/home/corr/.claude/plans/wild-crunching-flute.md`.
