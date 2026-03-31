import { create } from "zustand";
import { invoke } from "@/api/tauri";
import type { ServiceInfo } from "./sunshine";

export interface MoonlightNode {
  name: string;
  address: string;
  port: number;
  state: "online" | "offline" | "hosting" | "connected";
  source: "mdns" | "orrtellite" | "static";
  fingerprint: string | null;
  sunshine_available: boolean;
  moonlight_available: boolean;
  os: string | null;
  encoder: string | null;
}

interface MoonlightState {
  status: ServiceInfo | null;
  nodes: MoonlightNode[];
  connectedTo: string | null;
  loading: boolean;
  error: string | null;

  fetchStatus: () => Promise<void>;
  fetchNodes: () => Promise<void>;
  connect: (address: string, app?: string, windowed?: boolean, resolution?: string) => Promise<void>;
  disconnect: () => Promise<void>;
}

export const useMoonlightStore = create<MoonlightState>((set) => ({
  status: null,
  nodes: [],
  connectedTo: null,
  loading: false,
  error: null,

  fetchStatus: async () => {
    try {
      const status = (await invoke("get_moonlight_status")) as ServiceInfo;
      set({ status, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchNodes: async () => {
    try {
      const nodes = (await invoke("get_nodes")) as MoonlightNode[];
      set({ nodes, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  connect: async (address, app, windowed, resolution) => {
    set({ loading: true });
    try {
      await invoke("start_moonlight", {
        params: { address, app, windowed, resolution },
      });
      set({ connectedTo: address, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  disconnect: async () => {
    set({ loading: true });
    try {
      await invoke("stop_moonlight");
      set({ connectedTo: null, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },
}));
