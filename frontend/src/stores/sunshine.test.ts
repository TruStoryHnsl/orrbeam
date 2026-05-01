import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@/api/tauri";
import { useSunshineStore, type SunshineSettings } from "./sunshine";

vi.mock("@/api/tauri", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe("useSunshineStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSunshineStore.setState({
      status: null,
      gpu: null,
      monitors: [],
      settings: null,
      loading: false,
      error: null,
    });
  });

  it("stores fetched sunshine status", async () => {
    const status = {
      name: "Sunshine",
      status: "running" as const,
      version: "0.23.1",
      path: "/usr/bin/sunshine",
    };
    mockInvoke.mockResolvedValueOnce(status);

    await useSunshineStore.getState().fetchStatus();

    expect(mockInvoke).toHaveBeenCalledWith("get_sunshine_status");
    expect(useSunshineStore.getState().status).toEqual(status);
  });

  it("updates local settings after monitor change", async () => {
    const settings: SunshineSettings = {
      output_name: "DP-1",
      fps: 60,
      bitrate: 20000,
      encoder: "nvenc",
      codec: "h265",
      channels: 2,
    };
    useSunshineStore.setState({ settings });
    mockInvoke.mockResolvedValueOnce(null);

    await useSunshineStore.getState().setMonitor("HDMI-1");

    expect(mockInvoke).toHaveBeenCalledWith("set_sunshine_monitor", {
      monitor: "HDMI-1",
    });
    expect(useSunshineStore.getState().settings?.output_name).toBe("HDMI-1");
  });
});
