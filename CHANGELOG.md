# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Contributor guide for local development, branch conventions, and release expectations
- Architecture reference covering crate boundaries, IPC flow, and mesh topology

### Changed

- README now matches the current repo structure, platform matrix, and quickstart flow
- Project metadata now consistently declares the MIT license across root and frontend manifests

## [0.1.0] - 2026-04-23

### Added

- Tauri v2 workspace scaffold with Rust backend crates and React frontend
- Platform abstraction crate for Sunshine/Moonlight detection and process management
- Core crate for config, node identity, peer metadata, TLS, and wire helpers
- Network crate for discovery, mutual trust, and signed control-plane routes
- Two-panel Sunshine and Moonlight UI with mesh status and tray integration
- Pairing workflow for initiating and accepting remote trust and stream setup

### Changed

- Archived the earlier Python prototype under `v1/` while new development moved to the Tauri workspace
