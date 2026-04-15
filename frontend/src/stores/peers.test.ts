import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@/api/tauri";
import { usePeersStore } from "./peers";

vi.mock("@/api/tauri", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe("usePeersStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    usePeersStore.setState({
      peers: [],
      inboundMutualTrust: [],
      loading: false,
      error: null,
    });
  });

  it("stores the trusted peer list", async () => {
    const peers = [
      {
        name: "orrpheus",
        ed25519_fingerprint: "deadbeef",
        ed25519_public_key_b64: "BBBB",
        cert_sha256: "cafe",
        address: "100.64.0.4",
        control_port: 47782,
        permissions: {
          can_query_status: true,
          can_start_sunshine: true,
          can_stop_sunshine: true,
          can_submit_pin: true,
          can_list_peers: true,
        },
        tags: ["owned"],
        added_at: "2026-04-09T13:00:00Z",
        last_seen_at: "2026-04-09T15:12:34Z",
        note: "test peer",
      },
    ];
    mockInvoke.mockResolvedValueOnce(peers);

    await usePeersStore.getState().fetch();

    expect(mockInvoke).toHaveBeenCalledWith("list_trusted_peers");
    expect(usePeersStore.getState().peers).toEqual(peers);
    expect(usePeersStore.getState().loading).toBe(false);
  });

  it("stores inbound mutual trust requests", async () => {
    const inbound = [
      {
        request_id: "550e8400-e29b-41d4-a716-446655440000",
        initiator_name: "orrion",
        initiator_fingerprint: "abcd1234",
        note: "work machine",
        created_at: "2026-04-09T15:00:00Z",
      },
    ];
    mockInvoke.mockResolvedValueOnce(inbound);

    await usePeersStore.getState().fetchInbound();

    expect(mockInvoke).toHaveBeenCalledWith("list_inbound_mutual_trust_requests");
    expect(usePeersStore.getState().inboundMutualTrust).toEqual(inbound);
  });
});
