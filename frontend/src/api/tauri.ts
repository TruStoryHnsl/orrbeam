/// Tauri IPC wrapper with browser-mode mocks for dev iteration.

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

// Lazy-load the Tauri invoke function on first call
let cachedInvoke: InvokeFn | null = null;

async function getTauriInvoke(): Promise<InvokeFn> {
  if (!cachedInvoke) {
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    cachedInvoke = tauriInvoke as InvokeFn;
  }
  return cachedInvoke;
}

const mocks: Record<string, unknown> = {
  get_platform_info: {
    os: "linux",
    os_version: "CachyOS",
    display_server: "wayland",
    hostname: "orrion",
  },
  get_gpu_info: {
    name: "NVIDIA GeForce RTX 3070",
    encoder: "NVENC",
    driver: "565.57.01",
  },
  get_monitors: [
    { name: "DP-1", resolution: "2560x1440", refresh_rate: 165, primary: true },
    { name: "HDMI-1", resolution: "1920x1080", refresh_rate: 60, primary: false },
  ],
  get_sunshine_status: {
    name: "Sunshine",
    status: "running",
    version: "0.23.1",
    path: "/usr/bin/sunshine",
  },
  get_moonlight_status: {
    name: "Moonlight",
    status: "installed",
    version: "6.0.1",
    path: "/usr/bin/moonlight-qt",
  },
  get_nodes: [
    {
      name: "orrpheus",
      address: "100.66.55.59",
      port: 47782,
      state: "online",
      source: "orrtellite",
      fingerprint: "a1b2c3d4e5f6a7b8",
      sunshine_available: true,
      moonlight_available: true,
      os: "macos",
      encoder: "VideoToolbox",
    },
    {
      name: "mbp15",
      address: "192.168.1.110",
      port: 47782,
      state: "offline",
      source: "mdns",
      fingerprint: null,
      sunshine_available: true,
      moonlight_available: true,
      os: "linux",
      encoder: "VAAPI",
    },
  ],
  get_node_count: 2,
  get_config: {
    node_name: "orrion",
    discovery_enabled: true,
    mdns_enabled: true,
    orrtellite_enabled: true,
    orrtellite_url: "https://hs.orrtellite.orrgate.com",
    orrtellite_api_key: "",
    sunshine_path: null,
    moonlight_path: null,
    static_nodes: [],
  },
  get_identity: {
    fingerprint: "a1b2c3d4e5f6a7b8",
    public_key: [],
  },
};

function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  console.log(`[mock] invoke: ${cmd}`, args);
  if (cmd in mocks) return Promise.resolve(mocks[cmd]);
  return Promise.reject(new Error(`[mock] unknown command: ${cmd}`));
}

export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if (isTauri) {
    const fn = await getTauriInvoke();
    return fn(cmd, args);
  }
  return mockInvoke(cmd, args);
}
