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

export interface SunshineSettings {
  output_name: string | null;
  fps: number | null;
  bitrate: number | null;
  encoder: string | null;
  codec: string | null;
  channels: number | null;
}

interface SunshineState {
  status: ServiceInfo | null;
  gpu: GpuInfo | null;
  monitors: MonitorInfo[];
  settings: SunshineSettings | null;
  loading: boolean;
  error: string | null;

  fetchStatus: () => Promise<void>;
  fetchGpu: () => Promise<void>;
  fetchMonitors: () => Promise<void>;
  fetchSettings: () => Promise<void>;
  setMonitor: (name: string) => Promise<void>;
  updateSettings: (settings: SunshineSettings) => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
}

export const useSunshineStore = create<SunshineState>((set) => ({
  status: null,
  gpu: null,
  monitors: [],
  settings: null,
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

  fetchSettings: async () => {
    try {
      const settings = (await invoke("get_sunshine_settings")) as SunshineSettings;
      set({ settings, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setMonitor: async (name: string) => {
    try {
      await invoke("set_sunshine_monitor", { monitor: name });
      set((s) => ({
        settings: s.settings ? { ...s.settings, output_name: name } : null,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  updateSettings: async (settings: SunshineSettings) => {
    try {
      await invoke("set_sunshine_settings", { settings });
      set({ settings, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  start: async () => {
    set({ loading: true });
    try {
      await invoke("start_sunshine");
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
