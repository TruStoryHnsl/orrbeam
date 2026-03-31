import { create } from "zustand";
import { invoke } from "@/api/tauri";

export interface PlatformInfo {
  os: string;
  os_version: string | null;
  display_server: string | null;
  hostname: string;
}

export interface PublicIdentity {
  fingerprint: string;
  public_key: number[];
}

interface PlatformState {
  info: PlatformInfo | null;
  identity: PublicIdentity | null;

  fetchInfo: () => Promise<void>;
  fetchIdentity: () => Promise<void>;
}

export const usePlatformStore = create<PlatformState>((set) => ({
  info: null,
  identity: null,

  fetchInfo: async () => {
    const info = (await invoke("get_platform_info")) as PlatformInfo;
    set({ info });
  },

  fetchIdentity: async () => {
    const identity = (await invoke("get_identity")) as PublicIdentity;
    set({ identity });
  },
}));
