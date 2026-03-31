import { create } from "zustand";
import { invoke } from "@/api/tauri";

export interface ServiceInfo {
  name: string;
  status: "installed" | "running" | "not_installed" | "unknown";
  version: string | null;
  path: string | null;
}

export interface GpuInfo {
  name: string;
  encoder: string;
  driver: string | null;
}

export interface MonitorInfo {
  name: string;
  resolution: string;
  refresh_rate: number | null;
  primary: boolean;
}

interface SunshineState {
  status: ServiceInfo | null;
  gpu: GpuInfo | null;
  monitors: MonitorInfo[];
  loading: boolean;
  error: string | null;

  fetchStatus: () => Promise<void>;
  fetchGpu: () => Promise<void>;
  fetchMonitors: () => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
}

export const useSunshineStore = create<SunshineState>((set) => ({
  status: null,
  gpu: null,
  monitors: [],
  loading: false,
  error: null,

  fetchStatus: async () => {
    try {
      const status = (await invoke("get_sunshine_status")) as ServiceInfo;
      set({ status, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchGpu: async () => {
    try {
      const gpu = (await invoke("get_gpu_info")) as GpuInfo;
      set({ gpu });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchMonitors: async () => {
    try {
      const monitors = (await invoke("get_monitors")) as MonitorInfo[];
      set({ monitors });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  start: async () => {
    set({ loading: true });
    try {
      await invoke("start_sunshine");
      // Re-fetch status after a brief delay for process startup
      setTimeout(async () => {
        const status = (await invoke("get_sunshine_status")) as ServiceInfo;
        set({ status, loading: false, error: null });
      }, 1000);
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  stop: async () => {
    set({ loading: true });
    try {
      await invoke("stop_sunshine");
      const status = (await invoke("get_sunshine_status")) as ServiceInfo;
      set({ status, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },
}));
