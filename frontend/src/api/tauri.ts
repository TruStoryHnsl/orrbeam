/// Tauri IPC wrapper with browser-mode mocks for dev iteration.
///
/// Detection: Tauri v2 sets `window.__TAURI_INTERNALS__` before any JS runs
/// (injected via the webview preload script). We also check `window.isTauri`
/// as the official Tauri v2 detection signal. We check both to be safe.

import type { PeeringProgress } from "@/types/peers";

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

  // ── Peer management (WI-12) ──────────────────────────────────────────────

  list_trusted_peers: [
    {
      name: "orrpheus",
      ed25519_fingerprint: "a1b2c3d4e5f6a7b8",
      ed25519_public_key_b64: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      cert_sha256: "e3b0c44298fc1c149afbf4c8996fb924274a01d44c337b24e2b80f4903c7d90b",
      address: "100.66.55.59",
      control_port: 47782,
      permissions: {
        can_query_status: true,
        can_start_sunshine: true,
        can_stop_sunshine: true,
        can_submit_pin: true,
        can_list_peers: true,
      },
      tags: ["owned", "macos"],
      added_at: "2026-04-09T13:00:00Z",
      last_seen_at: "2026-04-09T15:12:34Z",
      note: "M1 Pro MacBook",
    },
  ],

  fetch_peer_hello: {
    node_name: "mock-peer",
    ed25519_fingerprint: "deadbeefcafebabe",
    ed25519_public_key_b64: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=",
    cert_sha256: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789ab",
    control_port: 47782,
    sunshine_available: true,
    moonlight_available: true,
    os: "macos",
    version: "orrbeam/1",
  },

  confirm_trusted_peer: null,
  remove_trusted_peer: true,
  update_peer_permissions: null,

  request_mutual_trust: {
    request_id: "550e8400-e29b-41d4-a716-446655440000",
    receiver_hello: {
      node_name: "mock-peer",
      ed25519_fingerprint: "deadbeefcafebabe",
      ed25519_public_key_b64: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=",
      cert_sha256: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789ab",
      control_port: 47782,
      sunshine_available: true,
      moonlight_available: true,
      os: "macos",
      version: "orrbeam/1",
    },
  },

  approve_mutual_trust_request: null,
  reject_mutual_trust_request: null,

  list_inbound_mutual_trust_requests: [
    {
      request_id: "660e8400-e29b-41d4-a716-446655440001",
      initiator_name: "orrion",
      initiator_fingerprint: "1234567890abcdef",
      note: "work machine",
      created_at: "2026-04-09T15:00:00Z",
    },
  ],

  connect_to_peer: null,
  remote_peer_status: {
    sunshine: { status: "running" },
    moonlight: { status: "installed" },
  },

  get_tls_fingerprint: {
    cert_sha256: "cafe1234abcdef5678cafe1234abcdef5678cafe1234abcdef5678cafe1234ab",
    ed25519_fingerprint: "abcd1234efgh5678",
    control_port: 47782,
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

// ── Event helpers ────────────────────────────────────────────────────────────

export type PeeringProgressCallback = (progress: PeeringProgress) => void;

/**
 * Subscribe to `peering:progress` events from the Tauri backend.
 * Returns an unlisten/cleanup function (call it to unsubscribe).
 *
 * In browser mock mode, listens for a custom DOM event on `window` so that
 * dev tooling can dispatch synthetic progress events for testing.
 */
export async function onPeeringProgress(cb: PeeringProgressCallback): Promise<() => void> {
  if (IS_TAURI) {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<PeeringProgress>("peering:progress", (event) => {
      cb(event.payload);
    });
    return unlisten;
  } else {
    // Mock mode: forward synthetic CustomEvents dispatched on window
    const handler = (e: Event) => {
      cb((e as CustomEvent<PeeringProgress>).detail);
    };
    window.addEventListener("peering:progress", handler);
    return () => window.removeEventListener("peering:progress", handler);
  }
}
