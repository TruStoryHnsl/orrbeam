//! End-to-end test stubs for the orrbeam mesh.
//!
//! All tests in this crate require physical hardware and are `#[ignore]`
//! by default. Run them explicitly with:
//!
//! ```bash
//! cargo test --workspace -- --ignored
//! ```

/// Verify two nodes can exchange identity and complete a pairing handshake.
///
/// Prerequisites:
/// - Two machines, each with orrbeam running and port 47782 reachable.
/// - Machine addresses configured in the test environment variables
///   `ORRBEAM_NODE_A` and `ORRBEAM_NODE_B`.
#[test]
#[ignore = "requires two physical orrbeam nodes on the LAN"]
fn test_node_pairing() {
    // TODO: Implement pairing handshake verification.
    // 1. GET /v1/hello from Node B to obtain its fingerprint.
    // 2. POST /v1/mutual-trust-request from Node A → Node B.
    // 3. Poll Node A's pending requests until approved.
    // 4. Verify both nodes report each other in GET /v1/peers.
    todo!("e2e: node pairing stub — implement when hardware is available")
}

/// Verify Sunshine starts on Node B, Moonlight connects from Node A,
/// the stream is active, and both stop cleanly.
///
/// Prerequisites:
/// - Two machines with Sunshine and moonlight-qt installed.
/// - Nodes already mutually trusted (run `test_node_pairing` first).
/// - A GPU with hardware encoding support (NVENC / VideoToolbox / AMF).
#[test]
#[ignore = "requires Sunshine + Moonlight installed on both nodes, with a real GPU"]
fn test_stream_start_stop() {
    // TODO: Implement stream lifecycle verification.
    // 1. POST /v1/sunshine/start on Node B (via signed request from Node A).
    // 2. POST /v1/pair/accept with the auto-generated PIN.
    // 3. Launch moonlight-qt on Node A targeting Node B.
    // 4. Assert the stream is active (check Sunshine status endpoint).
    // 5. Stop Moonlight and POST /v1/sunshine/stop.
    // 6. Assert clean shutdown (no dangling processes).
    todo!("e2e: stream start/stop stub — implement when hardware is available")
}

/// Verify bidirectional handoff: Node A→B stream followed by Node B→A stream.
///
/// Prerequisites:
/// - All prerequisites from `test_stream_start_stop`.
/// - Both nodes have Sunshine and Moonlight installed.
#[test]
#[ignore = "requires bidirectional Sunshine+Moonlight setup on both nodes"]
fn test_bidirectional_handoff() {
    // TODO: Implement bidirectional handoff verification.
    // Phase 1 (A→B):
    // 1. Node A starts streaming to Node B as in test_stream_start_stop.
    // 2. Assert stream is active A→B.
    // 3. Stop A→B stream cleanly.
    //
    // Phase 2 (B→A):
    // 4. Node B starts streaming to Node A (roles reversed).
    // 5. Assert stream is active B→A.
    // 6. Stop B→A stream cleanly.
    //
    // 7. Assert no processes leaked and registries are consistent.
    todo!("e2e: bidirectional handoff stub — implement when hardware is available")
}
