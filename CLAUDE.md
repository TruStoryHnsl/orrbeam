# Orrbeam

Unified Sunshine/Moonlight mesh — bidirectional remote desktop nodes.

## What it does

Every machine runs BOTH Sunshine (host) AND Moonlight (client), managed by a single daemon (`orrbeamd`). Any node can host to or connect from any other authorized node. The mesh is bidirectional and self-organizing.

## Architecture

```
orrbeam/
├── orrbeam/
│   ├── cli.py          # Click CLI: orrbeam {status,list,connect,ping,setup,...}
│   ├── daemon.py       # orrbeamd: async daemon with REST API on port 47782
│   ├── discovery.py    # mDNS (_orrbeam._tcp) + orrtellite (Headscale API)
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
orrbeam ping <node>  # Test connectivity
```

## Key ports

- **47782** — orrbeam daemon REST API
- **47984-47990** — Sunshine streaming
- **48010** — Sunshine RTSP

## Node discovery priority

1. **LAN mDNS** — `_orrbeam._tcp` service type, zero-config on same subnet
2. **orrtellite mesh** — queries Headscale API directly (no tailscale CLI needed), polls every 30s
3. **Static entries** — `~/.config/orrbeam/config.yaml` static_nodes list

## orrtellite integration

Orrbeam queries the self-hosted Headscale API at `orrtellite_url` with `orrtellite_api_key`. This returns all mesh peers with their 100.64.x.x IPs and online status. Orrbeam then probes each for an orrbeam daemon on port 47782.

Config (`~/.config/orrbeam/config.yaml`):
```yaml
orrtellite_enabled: true
orrtellite_url: https://hs.orrtellite.orrgate.com
orrtellite_api_key: <headscale-api-key>
```

No tailscale CLI dependency. Self-contained.

## Platform support

| Machine | OS | Host (Sunshine) | Client (Moonlight) | Service |
|---------|-----|-----------------|-------------------|---------|
| orrion | CachyOS (Linux) | NVENC (RTX 3070) | moonlight-qt | systemd user |
| orrpheus | macOS (M1 Pro) | VideoToolbox | Moonlight.app | launchd |
| mbp15 | Ubuntu 24.04 | VAAPI | flatpak Moonlight | systemd user |
| iOS/iPad | iOS | N/A | Companion app | N/A |

## Dev notes

- Daemon uses aiohttp for lightweight async HTTP
- CLI talks to daemon via localhost REST API (urllib, no extra deps)
- Platform layer is ABC-based — `get_platform()` auto-selects at import time
- Config stored at `~/.config/orrbeam/` (Linux) or `~/Library/Application Support/orrbeam/` (macOS)
- Identity keypair at `~/.local/share/orrbeam/identity/` (Linux) or same App Support dir (macOS)
- moonlight-qt `--version` opens GUI — use package manager for version detection
- `_run()` catches FileNotFoundError for cross-platform safety (nvidia-smi, vainfo may not exist)
