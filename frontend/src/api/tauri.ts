/// Tauri IPC wrapper with browser-mode mocks for dev iteration.
///
/// Detection: Tauri v2 sets `window.__TAURI_INTERNALS__` before any JS runs
/// (injected via the webview preload script). We also check `window.isTauri`
/// as the official Tauri v2 detection signal. We check both to be safe.

function detectTauri(): boolean {
  if (typeof window === "undefined") return false;
  // Official Tauri v2 detection (set by the runtime)
  if ("isTauri" in window) return true;
  // Fallback: check for the internal IPC bridge
  if ("__TAURI_INTERNALS__" in window) return true;
  return false;
}

const IS_TAURI = detectTauri();

// Log detection result so we can debug in the webview console
console.log(`[orrbeam] Tauri detected: ${IS_TAURI}`);

type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

// Lazy-load the real Tauri invoke — only imported when actually in Tauri
let cachedInvoke: InvokeFn | null = null;

async function getTauriInvoke(): Promise<InvokeFn> {
  if (!cachedInvoke) {
    const mod = await import("@tauri-apps/api/core");
    cachedInvoke = mod.invoke as InvokeFn;
  }
  return cachedInvoke;
}

// ── Mocks (only used when running in a plain browser, not in Tauri) ──

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
  get_sunshine_settings: {
    output_name: "DP-1",
    fps: 60,
    bitrate: 20000,
    encoder: "nvenc",
    codec: "h265",
    channels: 2,
  },
  set_sunshine_settings: null,
  set_sunshine_monitor: null,
  pair_initiate: { pin: "4217", target: "192.168.1.100", started: true },
  pair_accept: true,
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
    sunshine_username: "sunshine",
    sunshine_password: "sunshine",
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
  if (cmd in mocks) return Promise.resolve(structuredClone(mocks[cmd]));
  return Promise.reject(new Error(`[mock] unknown command: ${cmd}`));
}

export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if (IS_TAURI) {
    try {
      const fn = await getTauriInvoke();
      return await fn(cmd, args);
    } catch (e) {
      console.error(`[tauri] invoke ${cmd} failed:`, e);
      throw e;
    }
  }
  return mockInvoke(cmd, args);
}
