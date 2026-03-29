# Orrbeam Apple Companion App — Implementation Plan

## Overview

Native SwiftUI app for iOS and iPad that serves as a Moonlight client companion within the orrbeam mesh. The app handles node discovery, connection management, and launches Moonlight streams. It does NOT reimplement streaming — it works alongside the existing Moonlight iOS app.

## What the app does

1. **Discovers orrbeam nodes** on the local network (Bonjour/mDNS) and via Tailscale
2. **Shows a node list** with status (online/offline/hosting), GPU info, and capabilities
3. **Initiates connections** — taps a node to connect via Moonlight
4. **Manages pairing** — handles the Sunshine PIN pairing flow
5. **Shows connection history** and favorited nodes

## What the app does NOT do

- Stream video/audio (Moonlight handles this)
- Run Sunshine (iOS cannot host)
- Replace the Moonlight app (complements it)

## Architecture

```
OrrbeamApp/
├── OrrbeamApp.swift                 # @main entry point
├── Models/
│   ├── OrrbeamNode.swift            # Node model (mirrors Python Node)
│   ├── NodeState.swift              # Online/Offline/Hosting/Connected enum
│   └── MeshConfig.swift             # App configuration + persistence
├── Services/
│   ├── BonjourDiscovery.swift       # NetServiceBrowser for _orrbeam._tcp
│   ├── TailscaleDiscovery.swift     # Poll Tailscale VPN peers
│   ├── NodeAPIClient.swift          # HTTP client for orrbeam daemon REST API
│   └── MoonlightLauncher.swift      # Open moonlight:// URLs or Moonlight app
├── ViewModels/
│   ├── MeshViewModel.swift          # Main state: node list, discovery status
│   └── NodeDetailViewModel.swift    # Single node: status, connect, pair
├── Views/
│   ├── MeshView.swift               # Main node list (grid on iPad, list on iPhone)
│   ├── NodeCard.swift               # Node card component (name, status, GPU)
│   ├── NodeDetailView.swift         # Detail: connect button, pairing, apps
│   ├── SettingsView.swift           # Config: Tailscale toggle, static nodes
│   └── PairingSheet.swift           # PIN entry sheet for Sunshine pairing
├── Assets.xcassets/
└── Info.plist
```

## Key Technical Decisions

### 1. Node Discovery via Bonjour (NWBrowser)

iOS has native Bonjour support via Network.framework's `NWBrowser`. This discovers the same `_orrbeam._tcp` mDNS services that the Python daemon broadcasts.

```swift
let browser = NWBrowser(for: .bonjour(type: "_orrbeam._tcp", domain: nil), using: .tcp)
browser.stateUpdateHandler = { state in ... }
browser.browseResultsChangedHandler = { results, changes in
    // Each result contains the node name and TXT records (fingerprint, version)
}
browser.start(queue: .main)
```

### 2. Tailscale Discovery

Two approaches, in order of preference:

**A. Tailscale iOS SDK** (if available): The Tailscale iOS app exposes a local API. If the user has Tailscale installed, query its local API for peers.

**B. Manual probe**: If Tailscale IPs are configured in settings, probe each for an orrbeam daemon on port 47782. This is the fallback.

### 3. Moonlight Integration

Moonlight for iOS supports URL schemes. To launch a stream:

```swift
// Option A: URL scheme (if Moonlight supports it)
if let url = URL(string: "moonlight://stream/\(node.address)") {
    UIApplication.shared.open(url)
}

// Option B: Clipboard + app switch
// Copy the address to clipboard, prompt user to open Moonlight
UIPasteboard.general.string = node.address
```

**Research needed on orrpheus**: Check if Moonlight iOS actually supports a URL scheme. If not, the app will need to use the clipboard approach or embed Moonlight's open-source streaming code directly.

### 4. Communication with orrbeam daemon

The app talks to the daemon's REST API over the network (not localhost — the daemon runs on the remote machine being controlled):

```
GET  /health          → { status, service, node }
GET  /api/status      → { node_name, fingerprint, sunshine, moonlight, ... }
GET  /api/nodes       → { nodes: [...] }
POST /api/connect     → { node, app }
```

But for discovery and node listing, the app does its OWN Bonjour scanning — it doesn't need a daemon to list nodes. The daemon API is used for:
- Getting detailed status of a specific node
- Triggering pairing
- Getting the app list from a Sunshine host

### 5. iPad vs iPhone Layout

```
iPhone:                          iPad:
┌──────────────┐                ┌────────────────────────────────────┐
│ ≡ Orrbeam    │                │ ≡ Orrbeam          Settings ⚙     │
├──────────────┤                ├──────────┬─────────────────────────┤
│ ● orrion     │                │ ● orrion │  orrion                │
│   RTX 3070   │                │          │  192.168.1.152:47782   │
│   online     │                │ ● orrphe │  RTX 3070 · NVENC      │
├──────────────┤                │   us     │  Wayland · online      │
│ ● orrpheus   │                │          │                         │
│   M1 Pro     │                │ ○ mbp15  │  [Connect ▶]           │
│   online     │                │          │  [Pair 🔑]             │
├──────────────┤                │          │                         │
│ ○ mbp15      │                │          │  Apps:                  │
│   offline    │                │          │  · Desktop              │
└──────────────┘                │          │  · Steam                │
                                └──────────┴─────────────────────────┘
```

Use `NavigationSplitView` for automatic iPhone/iPad adaptation.

## Implementation Phases (for orrpheus session)

### Phase 1: Xcode Project + Bonjour Discovery
1. Create Xcode project with SwiftUI lifecycle
2. Set up `NWBrowser` for `_orrbeam._tcp` discovery
3. Build `OrrbeamNode` model
4. Build `MeshView` with live node list
5. Test: start orrbeamd on orrion, see it appear on iPhone/iPad

### Phase 2: Node Details + Status
1. Build `NodeAPIClient` (async URLSession calls to daemon REST API)
2. Build `NodeDetailView` with status display
3. Pull GPU info, Sunshine status, display type from daemon
4. Show online/offline state with live updates (poll every 5s)

### Phase 3: Moonlight Integration
1. Research Moonlight iOS URL schemes on orrpheus
2. Implement `MoonlightLauncher` (URL scheme or clipboard fallback)
3. Add "Connect" button that launches Moonlight stream
4. Add app selection (Desktop, Steam, etc.) from Sunshine's app list

### Phase 4: Pairing
1. Build `PairingSheet` for PIN entry
2. Implement pairing API call to target node's Sunshine instance
3. Store trusted node fingerprints in Keychain

### Phase 5: Polish
1. iPad split view layout
2. Connection history (UserDefaults or SwiftData)
3. Settings view (Tailscale toggle, manual nodes)
4. App icon and launch screen

## Prerequisites for orrpheus session

- [ ] Xcode installed on orrpheus
- [ ] Apple Developer account logged in (for device deployment)
- [ ] orrbeamd running on orrion (for testing discovery)
- [ ] Moonlight installed on test iOS device
- [ ] orrpheus and orrion on same network (LAN or Tailscale)

## Dependencies

- **Network.framework** (system) — Bonjour/NWBrowser
- **SwiftUI** (system) — UI
- **Foundation** (system) — URLSession for REST API
- No third-party dependencies needed for v1

## macOS-Specific Notes

The macOS version of orrbeam runs the Python daemon (orrbeamd via launchd). The Swift app is iOS/iPad only. On macOS, users use the CLI (`orrbeam status`, `orrbeam connect`, etc.) — same as Linux.

If we later want a macOS menu bar app, it would share the SwiftUI views from the iOS app via a multiplatform target. But that's post-v1.
