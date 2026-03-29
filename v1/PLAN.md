# Orrbeam — Master Development Plan

A unified Sunshine/Moonlight mesh manager that turns every participating machine into a bidirectional remote desktop node. Any node can host to or connect from any other authorized node — replacing the traditional one-way client/server model with a symmetric mesh.

## Open Conflicts
None yet.

## Architecture

### Core Concept

Traditional Sunshine+Moonlight: one machine runs Sunshine (host), another runs Moonlight (client). One-way tunnel. Each pair must be configured separately.

**Orrbeam**: Every machine runs BOTH Sunshine AND Moonlight, managed by a single wrapper application. The wrapper presents a unified node list — click any node to connect TO it, or see who's connected to YOU. The mesh is bidirectional and self-organizing.

```
Traditional:
  Machine A (Sunshine) ←── Machine B (Moonlight)
  One-way. B sees A's screen. A cannot see B.

Orrbeam:
  Machine A (Sunshine + Moonlight) ←→ Machine B (Sunshine + Moonlight)
  Either can host. Either can connect. Roles are dynamic.
```

### What orrbeam actually manages

1. **Local Sunshine instance** — ensures it's running, configured, ports open
2. **Local Moonlight instance** — available for outbound connections
3. **Node registry** — knows about all other orrbeam nodes on the network (via mDNS, orrtellite, or manual config)
4. **Pairing** — automates the Sunshine PIN pairing process between nodes
5. **Connection UI** — single interface to browse available nodes and connect/disconnect

### Node Discovery

Nodes find each other via (in priority order):
1. **orrtellite mesh** — if machines are on the orrtellite mesh, use Headscale mesh IPs
2. **LAN mDNS** — broadcast `_orrbeam._tcp` service for local discovery
3. **Manual config** — `~/.config/orrbeam/nodes.yaml` for static entries

### Components

**orrbeam daemon (`orrbeamd`)**
- Runs on each machine as a systemd service
- Manages local Sunshine and Moonlight installations
- Broadcasts availability via mDNS
- Listens for connection requests from other nodes
- Handles auto-pairing (pre-shared keys between trusted nodes)
- REST API on localhost for the TUI/CLI to query

**orrbeam CLI/TUI (`orrbeam`)**
- Terminal interface showing all discovered nodes
- Status: online/offline, currently hosting, currently connected
- Connect to a node: `orrbeam connect <node-name>`
- List nodes: `orrbeam list`
- Pair with a new node: `orrbeam pair <node-name>`
- Works over SSH (TUI mode)

**orrbeam installer (`orrbeam setup`)**
- Detects OS and display server (X11/Wayland/macOS)
- Installs Sunshine and Moonlight if not present
- Configures Sunshine for headless hosting if no display
- Configures firewall rules (Sunshine ports: 47984-47990, 48010)
- Registers the systemd service
- Generates a node identity (keypair for mesh auth)

### Compatibility with standalone Sunshine/Moonlight

Orrbeam nodes can connect to ANY traditional Sunshine host (not just other orrbeam nodes). A traditional Moonlight client can connect to an orrbeam node's Sunshine instance. Orrbeam is a superset, not a replacement.

### Technical Stack
- **Python** — daemon + CLI + installer (matches existing toolchain)
- **Textual** or **rich** for TUI
- **zeroconf** library for mDNS discovery
- **systemd** for daemon management
- Wraps existing Sunshine and Moonlight binaries (does NOT reimplement streaming)

### Machine Compatibility

| Machine | Display | Sunshine viable? | Moonlight viable? | Notes |
|---------|---------|-----------------|-------------------|-------|
| orrion | GPU (RTX 3070) + Wayland | Yes (NVENC) | Yes | Primary workstation |
| orrpheus | macOS (M1 Pro) | Yes (VideoToolbox) | Yes | macOS native Moonlight |
| mbp15 | Intel (i5-5257U) + X11/Wayland | Yes (VAAPI) | Yes | May struggle with encoding |
| cb17 | Intel (N3060) + X11 | Marginal (no HW encode) | Yes (client only) | Too weak to host, good client |
| orrgate | Headless (1050 Ti) | Yes (NVENC, headless) | No (no display to view) | Can host VMs/desktops |

## Feature Roadmap

1. [ ] **Installer script** — detect OS, install Sunshine + Moonlight, configure firewall, create systemd service
2. [ ] **Node identity** — keypair generation, node config file
3. [ ] **Daemon** — manage Sunshine/Moonlight processes, REST API, mDNS broadcast
4. [ ] **CLI** — `orrbeam list`, `orrbeam connect`, `orrbeam pair`, `orrbeam status`
5. [ ] **Auto-pairing** — pre-shared key exchange between trusted nodes (skip manual PIN)
6. [ ] **TUI** — interactive node browser with connect/disconnect
7. [ ] **Headless hosting** — virtual display creation for machines without monitors
8. [ ] **Integration with orrtellite** — use Headscale IPs for node discovery when available

## Recent Changes
- 2026-03-26: Initial plan created from user feedback in general admin session
