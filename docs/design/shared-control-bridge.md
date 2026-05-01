# Shared-Control Bridge (SFU Relay) — Feasibility Analysis

**Status**: Spike / Pre-implementation  
**Date**: 2026-04-15  
**Decision required**: Direct mode only vs. optional headless relay mode

---

## Problem

In shared-control mode, the host machine (e.g., orrion) runs Sunshine and streams to N participants simultaneously. Each participant receives an independent unicast stream from Sunshine. As N grows, the host's upstream bandwidth scales linearly:

```
N participants × bitrate_per_stream = total_upstream
4 participants × 20 Mbps (1080p60) = 80 Mbps total upstream
```

For LAN use this is fine (1 Gbps). For WAN/VPN (residential 100–500 Mbps uplink), 4 participants already approaches the limit.

An SFU (Selective Forwarding Unit) relay node could receive **one** stream from the host and re-distribute it to all participants, saving the host's upstream bandwidth at the cost of added latency.

---

## Mode Comparison

### Direct Mode (current architecture)

```
Host (Sunshine)  ──20 Mbps──►  Participant A (Moonlight)
                 ──20 Mbps──►  Participant B (Moonlight)
                 ──20 Mbps──►  Participant C (Moonlight)
                 ──20 Mbps──►  Participant D (Moonlight)
                               ─────────────────────────
                               Total host upstream: 80 Mbps
```

- **Latency**: Host → Participant (1 hop). Glass-to-glass: ~15–50 ms on LAN.
- **Host bandwidth**: O(N × bitrate).
- **Relay bandwidth**: None.
- **Complexity**: Low — no relay infrastructure.
- **Constraint**: Host upstream is the bottleneck for WAN use cases.

### SFU Relay Mode

```
Host (Sunshine) ──20 Mbps──► Relay Node ──20 Mbps──► Participant A
                                         ──20 Mbps──► Participant B
                                         ──20 Mbps──► Participant C
                                         ──20 Mbps──► Participant D
                               ─────────────────────────────────────
                               Host upstream: 20 Mbps (fixed)
                               Relay downstream + upstream: 20 + 80 Mbps
```

- **Latency**: Host → Relay → Participant (2 hops). Added latency = relay processing time.
- **Host bandwidth**: O(1) — only one stream to relay.
- **Relay bandwidth**: O(N × bitrate) upstream + 1× bitrate downstream.
- **Complexity**: High — relay node required with significant hardware.

---

## Latency Tax Analysis

An SFU relay adds two operations:

1. **Forwarding (no re-encode)**: The relay receives the H.264/HEVC bitstream from Sunshine and forwards it to participants without decoding or re-encoding.
   - Added latency: ~1–5 ms (network jitter buffer + forwarding overhead).
   - This is the preferred approach if Moonlight supports connecting to a relay that mirrors the stream.

2. **Re-encoding (decode + encode)**: The relay decodes the stream and re-encodes it (e.g., to transcode resolution or bitrate per participant).
   - Added latency: 10–30 ms (decode + encode round trip).
   - Increases glass-to-glass from ~50 ms to ~60–80 ms on LAN.

**Crossover point (direct vs. SFU forwarding)**:

| Participants | Direct host upstream | SFU relay useful? |
|--------------|---------------------|-------------------|
| 1–2 | 20–40 Mbps | No — direct is simpler and lower latency |
| 3–4 | 60–80 Mbps | Marginal — LAN is fine; WAN starts to strain |
| 5–8 | 100–160 Mbps | Yes — WAN hosts benefit from relay |
| 9+ | 180+ Mbps | Yes — relay is effectively required for WAN |

**Recommendation**: Direct mode is superior for ≤4 participants on LAN. SFU becomes beneficial at ≥5 participants on WAN, or at ≥3 participants on hosts with <60 Mbps uplink.

---

## Conflict with "No Headless Mode" Architectural Constraint

Orrbeam's current architecture explicitly states:

> **No headless mode**: Desktop-only application (no orrgate, no servers)

An SFU relay node fundamentally requires running as a headless service — it receives streams and forwards them without any local display or GUI. This directly contradicts the No-headless constraint.

### Two Options

**Option A: Maintain No-Headless — Direct Mode Only**

- Shared-control is limited to direct mode.
- Participant cap is determined by host upstream bandwidth.
- No relay infrastructure needed.
- Simple, aligned with current architecture.
- **Limitation**: WAN shared-control with >4 participants is impractical on typical residential ISPs.
- **Verdict**: Correct for v1. Defer SFU to a future headless relay mode.

**Option B: Introduce Lightweight Headless Relay Mode**

- Add an optional `orrbeam-relay` binary (separate from the Tauri app).
- Relay runs on orrgate or another server node as a systemd service.
- Orrbeam GUI nodes can designate a relay for a shared-control session.
- Relay is a new architectural surface: authentication, management, updates.
- **Cost**: Significant scope increase. Requires separate installer, systemd unit, firewall rules, version management.
- **Benefit**: Enables WAN shared-control for >4 participants.
- **Verdict**: Valid future milestone (post-v1). Should be tracked as a separate work item, not in this sprint.

---

## Recommendation

**For the current sprint and v1**: Implement shared-control in **direct mode only** (Option A).

Rationale:
1. Direct mode covers the primary use case (LAN co-play at orrion with ≤4 participants).
2. Adding a relay violates the No-headless architectural principle and more than doubles the scope.
3. The latency tax of an SFU (even pure forwarding) is non-trivial for competitive input scenarios.
4. WAN shared-control with many participants is a v2+ feature.

**For v2+**: Evaluate `orrbeam-relay` as a separate lightweight daemon. It should be a distinct binary, not an extension of the Tauri app. The GUI app should be able to discover and delegate to a relay via the existing control-plane protocol (`/v1/relay/register` endpoint family).

---

## Implementation Note for Direct Mode

Even in direct mode, the architecture should be forward-compatible with relay by designing the session initiation protocol to include a relay address field (optional, null in v1). This allows v2 to introduce relay support without breaking the wire format.

```rust
pub struct SharedControlSession {
    pub session_id: Uuid,
    pub host_peer: String,
    /// Optional relay node. None = direct mode (v1).
    pub relay_peer: Option<String>,
    pub participants: Vec<String>,
    pub config: SharedControlConfig,
}
```

---

## Open Questions

- Does Moonlight support connecting to a stream forwarder/proxy that is not a real Sunshine instance? If not, pure forwarding is not feasible without protocol-level work.
- What is the minimum relay hardware spec? (orrgate CPU/RAM needs assessment if relay is pursued in v2.)
