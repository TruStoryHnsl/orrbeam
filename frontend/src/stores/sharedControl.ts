import { create } from "zustand";
import { invoke } from "@/api/tauri";

interface SharedControlState {
  /** Whether a shared-control session is currently active. */
  enabled: boolean;
  /** Names of all active participants in the session. */
  participants: string[];
  /** Whether an async operation is in progress. */
  loading: boolean;
  /** Last error message, or null. */
  error: string | null;

  /** Start the shared-control session on this node. */
  start: () => Promise<void>;
  /** Stop the shared-control session and release all virtual devices. */
  stop: () => Promise<void>;
  /** Add a named participant to the active session. */
  addParticipant: (name: string) => Promise<void>;
  /** Remove a named participant from the active session. */
  removeParticipant: (name: string) => Promise<void>;
  /** Refresh the participant list from the backend. */
  refresh: () => Promise<void>;
}

export const useSharedControlStore = create<SharedControlState>((set, get) => ({
  enabled: false,
  participants: [],
  loading: false,
  error: null,

  start: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("start_shared_control");
      const participants = (await invoke("list_sc_participants")) as string[];
      set({ enabled: true, participants, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  stop: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("stop_shared_control");
      set({ enabled: false, participants: [], loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  addParticipant: async (name: string) => {
    set({ loading: true, error: null });
    try {
      await invoke("add_sc_participant", { name });
      await get().refresh();
      set({ loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  removeParticipant: async (name: string) => {
    set({ loading: true, error: null });
    try {
      await invoke("remove_sc_participant", { name });
      await get().refresh();
      set({ loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  refresh: async () => {
    try {
      const participants = (await invoke("list_sc_participants")) as string[];
      set({ participants });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
