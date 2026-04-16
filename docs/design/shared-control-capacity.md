# Shared-Control Capacity — Benchmark Plan and Envelope

**Status**: Spike / Pre-implementation  
**Date**: 2026-04-15  
**Target hardware**: orrion (CachyOS Linux, RTX 3070, Ryzen CPU)

---

## Objective

Define the practical participant cap for shared-control mode on orrion, and determine the configuration field names that will expose these limits to the user.

---

## Hardware Baseline (orrion)

| Component | Spec |
|-----------|------|
| GPU | NVIDIA RTX 3070 (NVENC, 3rd-gen, 2× encode engines) |
| CPU | (AMD Ryzen, specific model TBD — see `/proc/cpuinfo`) |
| RAM | (TBD — `free -h`) |
| Network | Gigabit LAN (orrgate), WireGuard mesh via orrtellite |
| OS | CachyOS Linux (kernel 6.x, Wayland/KDE) |

---

## Metrics to Benchmark

### 1. CPU Overhead per Virtual Input Device

- **Tool**: `perf stat`, `htop`, `/proc/PID/stat`
- **Method**: Create N uinput devices, inject 1000 events/sec per device, measure ΔcPU% vs baseline (0 devices).
- **Expected**: <1% CPU per device up to 8 devices (uinput is kernel-handled).
- **Threshold**: CPU overhead for input alone must stay below 5% total for ≤8 participants.

### 2. GPU NVENC Encode Queue Depth

The RTX 3070 has 2 NVENC encode engines. Sunshine uses one NVENC session per stream.

- **Tool**: `nvidia-smi dmon -s u` (NVENC utilization), `nvtop`
- **Method**: Run Sunshine streaming at target resolution/FPS, measure NVENC utilization.
- **At 1080p60**: Expected 30–50% NVENC utilization per session on RTX 3070.
- **At 1440p60**: Expected 50–70% NVENC utilization.
- **Implication**: With 2 NVENC engines, orrion can sustain at most 2 concurrent Sunshine sessions before NVENC saturates. Shared-control with 1 Sunshine session (N participants all watching the same stream) does not multiply NVENC load — only 1 encode session regardless of participant count.

> **Key insight**: Shared-control does NOT require one encode session per participant. All participants watch the same encoded stream (multicast-style from one Sunshine session). NVENC load is O(1) w.r.t. participant count.

### 3. Bandwidth per Participant

Sunshine streams to each Moonlight client independently (unicast). Bandwidth scales linearly with connected Moonlight clients.

| Resolution | Bitrate (Sunshine default) | Bitrate per client |
|------------|---------------------------|-------------------|
| 1080p60    | ~20 Mbps                  | ~20 Mbps          |
| 1440p60    | ~40 Mbps                  | ~40 Mbps          |
| 4K60       | ~80 Mbps                  | ~80 Mbps          |

**LAN capacity (orrgate, 1 Gbps)**: ~50× 1080p60 clients before saturation.  
**WireGuard mesh (orrtellite)**: Individual node uplinks will be the bottleneck (typically 100–500 Mbps for residential ISPs).

**Recommended default**: 4 simultaneous participants at 1080p60 ≈ 80 Mbps total — well within LAN, marginal on typical WAN uplinks.

### 4. Latency Budget

Target: glass-to-glass latency ≤50 ms for acceptable interactive input.

| Stage | Expected latency |
|-------|----------------|
| Input capture on participant device | 1–5 ms |
| Network transit (LAN) | 0.5–2 ms |
| Orrbeam control-plane overhead | 1–3 ms |
| uinput kernel injection | <1 ms |
| Sunshine encode + stream | 10–30 ms |
| Network transit (stream, LAN) | 0.5–2 ms |
| Moonlight decode + display | 2–8 ms |
| **Total (LAN)** | **15–51 ms** |

LAN is within budget. WAN (VPN) adds 10–50 ms depending on geography, pushing glass-to-glass to 25–100 ms. This is acceptable for turn-based or casual co-op but borderline for fast-paced competitive input.

### 5. Benchmark Protocol

```bash
# Step 1: Baseline CPU/GPU at idle
nvidia-smi dmon -s u -d 1 -c 30 > baseline_nvenc.txt
mpstat 1 30 > baseline_cpu.txt

# Step 2: Sunshine streaming to 1 Moonlight client at 1080p60
# (start Sunshine, connect Moonlight, let stabilize 30s)
nvidia-smi dmon -s u -d 1 -c 60 > streaming_1client_nvenc.txt
mpstat 1 60 > streaming_1client_cpu.txt

# Step 3: Repeat with 2, 4 Moonlight clients

# Step 4: Add virtual uinput devices (1, 2, 4, 8) while streaming
# Inject synthetic events at 60Hz per device
# Measure: CPU delta, input latency (gettimeofday before/after inject)
```

---

## Recommended Default Limits

Based on analysis and expected benchmark results:

| Metric | Recommended default | Rationale |
|--------|-------------------|-----------|
| `max_participants` | 4 | ≤80 Mbps LAN, safe NVENC headroom |
| `target_fps` | 60 | Smooth interactive; 30 acceptable for bandwidth-limited |
| `target_resolution` | `"1920x1080"` | Safe default; 1440p for capable uplinks |
| `input_timeout_ms` | 100 | Disconnect virtual device if no input for 100ms (prevents stuck keys) |

---

## Config Field Names

These fields will be added to `Config` (or a new `SharedControlConfig` nested struct) in `crates/orrbeam-core/src/config.rs`:

```rust
pub struct SharedControlConfig {
    /// Maximum number of simultaneous shared-control participants.
    /// Default: 4. Range: 1–8.
    pub max_participants: u8,
    /// Target encode framerate for streaming sessions.
    /// Default: 60. Common values: 30, 60.
    pub target_fps: u8,
    /// Target streaming resolution as "WxH" string (e.g., "1920x1080").
    /// Default: "1920x1080".
    pub target_resolution: String,
    /// Milliseconds of input silence before a participant's virtual device
    /// is considered inactive and their slot may be reclaimed.
    /// Default: 100. Range: 16–5000.
    pub input_timeout_ms: u32,
    /// Input conflict resolution strategy.
    /// "last_write_wins" (default) or "priority_queue".
    pub input_conflict_strategy: String,
}
```

---

## Capacity Envelope Summary

| Scenario | Participants | NVENC | CPU | Bandwidth | Verdict |
|----------|-------------|-------|-----|-----------|---------|
| LAN, 1080p60 | 4 | ~40% | <5% | 80 Mbps | ✓ Recommended default |
| LAN, 1440p60 | 4 | ~60% | <5% | 160 Mbps | ✓ LAN only |
| WAN/VPN, 1080p60 | 2 | ~20% | <5% | 40 Mbps | ✓ Typical ISP uplink |
| LAN, 1080p60 | 8 | ~40% | <10% | 160 Mbps | ⚠ Bandwidth limited on WAN |
| LAN, 1440p60 | 8 | ~100% | <10% | 320 Mbps | ✗ NVENC saturated |

**Hard cap**: 8 participants (configurable down to 1). Above 4 at 1440p60, NVENC on RTX 3070 will be saturated and Sunshine may drop frames or refuse new connections.

---

## Open Questions

- Actual Ryzen CPU model on orrion needs confirming for precise core-count / thread scheduling analysis.
- Benchmark needs to run on actual hardware to validate NVENC utilization estimates.
- WireGuard (orrtellite) throughput per-tunnel needs measurement under Sunshine stream load.
