import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@/api/tauri";
import { usePlatformStore } from "./platform";

vi.mock("@/api/tauri", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe("usePlatformStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    usePlatformStore.setState({
      info: null,
      identity: null,
    });
  });

  it("stores fetched platform info", async () => {
    const info = {
      os: "linux",
      os_version: "CachyOS",
      display_server: "wayland",
      hostname: "orrion",
    };
    mockInvoke.mockResolvedValueOnce(info);

    await usePlatformStore.getState().fetchInfo();

    expect(mockInvoke).toHaveBeenCalledWith("get_platform_info");
    expect(usePlatformStore.getState().info).toEqual(info);
  });

  it("stores fetched identity", async () => {
    const identity = {
      fingerprint: "abcd1234",
      public_key: [1, 2, 3],
    };
    mockInvoke.mockResolvedValueOnce(identity);

    await usePlatformStore.getState().fetchIdentity();

    expect(mockInvoke).toHaveBeenCalledWith("get_identity");
    expect(usePlatformStore.getState().identity).toEqual(identity);
  });
});
