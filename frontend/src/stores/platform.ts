import { create } from "zustand";
import { invoke } from "@/api/tauri";

export interface PlatformInfo {
  os: string;
  os_version: string | null;
  display_server: string | null;
  hostname: string;
}

/**
 * Render an OS identifier as a user-friendly label.
 *
 * Audit (2026-04-27): the orrbeam frontend has NO hardcoded
 * Linux/macOS/Windows paths in user-visible strings — all platform-specific
 * resolution happens in the Rust backend (sunshine_conf::conf_path,
 * orrbeam-platform binary discovery, secure_file ACL helper). Components
 * that need to vary by OS should switch on `info.os` here.
 */
export function osDisplayName(os: string): string {
  switch (os) {
    case "linux":
      return "Linux";
    case "macos":
      return "macOS";
    case "windows":
      return "Windows";
    default:
      return os;
  }
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
    try {
      const info = (await invoke("get_platform_info")) as PlatformInfo;
      set({ info });
    } catch (e) {
      console.error("[platform] fetchInfo failed:", e);
    }
  },

  fetchIdentity: async () => {
    try {
      const identity = (await invoke("get_identity")) as PublicIdentity;
      set({ identity });
    } catch (e) {
      console.error("[platform] fetchIdentity failed:", e);
    }
  },
}));
