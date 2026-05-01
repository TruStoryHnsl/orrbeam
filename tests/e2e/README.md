# Orrbeam End-to-End Tests

Hardware-gated integration tests for the full orrbeam mesh stack.
All tests are `#[ignore]` by default and **will not run in CI**.

## Hardware Prerequisites

| Requirement | Details |
|-------------|---------|
| Two physical machines | Both must run orrbeam v2 with port 47782 reachable |
| LAN connectivity | Machines must be on the same network or connected via orrtellite mesh |
| Sunshine installed | Each machine needs Sunshine configured with a valid username/password |
| Moonlight installed | `moonlight-qt` (Linux), `Moonlight.app` (macOS), or Moonlight (Windows) |
| Hardware GPU | NVENC (NVIDIA), VideoToolbox (Apple Silicon/Intel Mac), or AMF (AMD) — software encoding will not pass stream quality checks |
| Mutual trust configured | Both nodes must have each other in their trusted peer store before stream tests |

## Environment Variables

```bash
export ORRBEAM_NODE_A="192.168.1.152:47782"   # orrion
export ORRBEAM_NODE_B="192.168.1.132:47782"   # orrpheus
```

## Running the Tests

```bash
# Run all e2e tests (skipped tests will be executed)
cargo test --workspace -- --ignored

# Run a specific e2e test
cargo test --package orrbeam-e2e test_node_pairing -- --ignored --nocapture

# List all e2e tests without running
cargo test --package orrbeam-e2e -- --list
```

## Test Order

Run in this order; later tests depend on earlier ones:

1. `test_node_pairing` — establishes mutual trust
2. `test_stream_start_stop` — verifies stream lifecycle
3. `test_bidirectional_handoff` — requires stream_start_stop passing

## Machines (orrbeam cluster)

| Node | Address | OS | GPU |
|------|---------|-----|-----|
| orrion | 192.168.1.152 | CachyOS (Linux) | RTX 3070 (NVENC) |
| orrpheus | 192.168.1.132 | macOS M1 Pro | VideoToolbox |
