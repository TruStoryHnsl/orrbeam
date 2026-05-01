import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@/api/tauri";
import { useMoonlightStore } from "./moonlight";

vi.mock("@/api/tauri", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe("useMoonlightStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useMoonlightStore.setState({
      status: null,
      nodes: [],
      connectedTo: null,
      loading: false,
      error: null,
    });
  });

  it("stores discovered nodes", async () => {
    const nodes = [
      {
        name: "orrpheus",
        address: "100.64.0.4",
        port: 47782,
        state: "online" as const,
        source: "orrtellite" as const,
        fingerprint: "deadbeef",
        sunshine_available: true,
        moonlight_available: true,
        os: "macos",
        encoder: "VideoToolbox",
      },
    ];
    mockInvoke.mockResolvedValueOnce(nodes);

    await useMoonlightStore.getState().fetchNodes();

    expect(mockInvoke).toHaveBeenCalledWith("get_nodes");
    expect(useMoonlightStore.getState().nodes).toEqual(nodes);
  });

  it("tracks the active connection target after connect", async () => {
    mockInvoke.mockResolvedValueOnce(null);

    await useMoonlightStore.getState().connect("100.64.0.4", "Desktop", true, "1920x1080");

    expect(mockInvoke).toHaveBeenCalledWith("start_moonlight", {
      params: {
        address: "100.64.0.4",
        app: "Desktop",
        windowed: true,
        resolution: "1920x1080",
      },
    });
    expect(useMoonlightStore.getState().connectedTo).toBe("100.64.0.4");
  });
});
