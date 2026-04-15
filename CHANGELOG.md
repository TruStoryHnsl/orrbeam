# Changelog

All notable changes to Orrbeam are documented here.

Format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning 2.0.0](https://semver.org/).

Releases are created with `/release orrbeam <major|minor|patch>` — see CONTRIBUTING.md.

---

## [Unreleased]

### Added
- Linter and formatter configuration (`rustfmt.toml`, `clippy.toml`, frontend ESLint + Prettier)
- GitHub issue and PR templates (`.github/ISSUE_TEMPLATE/`, `.github/pull_request_template.md`)
- `SECURITY.md` with disclosure policy and triage SLA
- Structured logging: replaced `println!`/`eprintln!` with `tracing` macros across all crates
- Persistent node registry (`NodeRegistry::load`/`save`, `~/.config/orrbeam/known_nodes.yaml`)
- `last_seen` timestamp on `Node`, offline-but-known nodes rendered in UI
- Tauri commands: `add_node`, `remove_node`, `list_nodes`

---

## [0.1.0] - 2026-04-14

Initial release of the Tauri v2 rewrite (v2 architecture, archived Python v1 to `v1/`).

### Added
- Cargo workspace with three library crates: `orrbeam-core`, `orrbeam-net`, `orrbeam-platform`
- `orrbeam-core`: Ed25519 identity, YAML config, node types, `NodeRegistry`, Sunshine conf read/write, TLS identity (self-signed cert), wire protocol (signed headers, nonce cache), trusted peer store
- `orrbeam-net`: mDNS discovery (`_orrbeam._tcp`), orrtellite (Headscale API) polling, Axum control plane HTTPS server (port 47782), `ControlClient`, `PinnedVerifier`
- `orrbeam-platform`: Platform abstraction trait with Linux, macOS, and Windows implementations; Sunshine and Moonlight process management
- `src-tauri`: Tauri v2 app with `AppState`, Tauri IPC commands (sunshine, moonlight, discovery, platform, settings, remote peers, pairing)
- React 19 frontend: Zustand stores, two-panel layout (Sunshine left / Moonlight right), mesh visualization, settings drawer (general, peers, about tabs)
- Node discovery: LAN mDNS + orrtellite Headscale mesh + static config entries
- Trusted-peer control plane: Ed25519 signatures, TOFU mutual trust, per-peer permissions
- System tray integration
- MIT license

[Unreleased]: https://github.com/TruStoryHnsl/orrbeam/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/TruStoryHnsl/orrbeam/releases/tag/v0.1.0
