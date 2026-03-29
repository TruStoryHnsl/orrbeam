# Orrbeam

Unified Sunshine/Moonlight mesh — bidirectional remote desktop nodes.

## What it does

Every machine runs BOTH Sunshine (host) AND Moonlight (client), managed by a single daemon (`orrbeamd`). Any node can host to or connect from any other authorized node. The mesh is bidirectional and self-organizing.

## Architecture

```
orrbeam/
├── orrbeam/
│   ├── cli.py          # Click CLI: orrbeam {status,list,connect,setup,...}
│   ├── daemon.py       # orrbeamd: async daemon with REST API on port 47782
│   ├── discovery.py    # mDNS (_orrbeam._tcp) + Tailscale peer scanning
│   ├── node.py         # Node model + NodeRegistry
│   ├── identity.py     # Ed25519 keypair for node identity
│   ├── config.py       # YAML config at ~/.config/orrbeam/config.yaml
│   └── platform/
│       ├── base.py     # Abstract Platform interface
│       ├── linux.py    # systemd, pacman/apt, nvidia-smi/vainfo
│       └── macos.py    # launchd, homebrew, VideoToolbox
└── apple/              # iOS/iPad SwiftUI companion app (planned)
```

## Quick start

```bash
pip install -e .
orrbeam setup        # Install Sunshine+Moonlight, configure firewall, register service
orrbeamd             # Start daemon (or let the service run it)
orrbeam status       # Check local node
orrbeam list         # See all mesh nodes
orrbeam connect <node>  # Stream from a node
```

## Key ports

- **47782** — orrbeam daemon REST API (localhost by default)
- **47984-47990** — Sunshine streaming
- **48010** — Sunshine RTSP

## Node discovery priority

1. Tailscale/orrtellite mesh (polls `tailscale status --json` every 30s)
2. LAN mDNS (`_orrbeam._tcp` service type)
3. Static entries in `~/.config/orrbeam/nodes.yaml`

## Platform support

| Machine | OS | Host (Sunshine) | Client (Moonlight) | Service |
|---------|-----|-----------------|-------------------|---------|
| orrion | CachyOS (Linux) | NVENC (RTX 3070) | moonlight-qt | systemd user |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app | launchd |
| mbp15 | Ubuntu 24.04 | VAAPI | moonlight-qt | systemd user |
| iOS/iPad | iOS | N/A | Companion app | N/A |

## Dev notes

- Daemon uses aiohttp for lightweight async HTTP
- CLI talks to daemon via localhost REST API (urllib, no extra deps)
- Platform layer is ABC-based — `get_platform()` auto-selects at import time
- Config stored at `~/.config/orrbeam/` (Linux) or `~/Library/Application Support/orrbeam/` (macOS)
- Identity keypair at `~/.local/share/orrbeam/identity/` (Linux) or same App Support dir (macOS)
