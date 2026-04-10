import { create } from "zustand";
import { invoke } from "@/api/tauri";
import type {
  TrustedPeer,
  HelloPayload,
  PeerDraft,
  PeerPermissions,
  MutualTrustInitResult,
  PendingMutualTrustSummary,
} from "@/types/peers";

interface PeersState {
  peers: TrustedPeer[];
  inboundMutualTrust: PendingMutualTrustSummary[];
  loading: boolean;
  error: string | null;

  // Actions
  fetch: () => Promise<void>;
  fetchInbound: () => Promise<void>;
  fetchPeerHello: (address: string, port: number) => Promise<HelloPayload>;
  confirmPeer: (draft: PeerDraft) => Promise<void>;
  removePeer: (name: string) => Promise<void>;
  updatePermissions: (name: string, permissions: PeerPermissions) => Promise<void>;
  requestMutualTrust: (address: string, port: number, note?: string) => Promise<MutualTrustInitResult>;
  approveMutualTrust: (requestId: string) => Promise<void>;
  rejectMutualTrust: (requestId: string) => Promise<void>;
  connectTo: (peerName: string) => Promise<void>;
  remoteStatus: (peerName: string) => Promise<unknown>;
}

export const usePeersStore = create<PeersState>((set) => ({
  peers: [],
  inboundMutualTrust: [],
  loading: false,
  error: null,

  fetch: async () => {
    try {
      set({ loading: true, error: null });
      const peers = (await invoke("list_trusted_peers")) as TrustedPeer[];
      set({ peers, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  fetchInbound: async () => {
    try {
      const inboundMutualTrust = (await invoke(
        "list_inbound_mutual_trust_requests"
      )) as PendingMutualTrustSummary[];
      set({ inboundMutualTrust });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchPeerHello: async (address: string, port: number) => {
    return (await invoke("fetch_peer_hello", { address, port })) as HelloPayload;
  },

  confirmPeer: async (draft: PeerDraft) => {
    await invoke("confirm_trusted_peer", { draft });
    const peers = (await invoke("list_trusted_peers")) as TrustedPeer[];
    set({ peers });
  },

  removePeer: async (name: string) => {
    await invoke("remove_trusted_peer", { name });
    const peers = (await invoke("list_trusted_peers")) as TrustedPeer[];
    set({ peers });
  },

  updatePermissions: async (name: string, permissions: PeerPermissions) => {
    await invoke("update_peer_permissions", { name, permissions });
    const peers = (await invoke("list_trusted_peers")) as TrustedPeer[];
    set({ peers });
  },

  requestMutualTrust: async (address: string, port: number, note?: string) => {
    return (await invoke("request_mutual_trust", {
      address,
      port,
      note,
    })) as MutualTrustInitResult;
  },

  approveMutualTrust: async (requestId: string) => {
    await invoke("approve_mutual_trust_request", { requestId });
    const peers = (await invoke("list_trusted_peers")) as TrustedPeer[];
    const inboundMutualTrust = (await invoke(
      "list_inbound_mutual_trust_requests"
    )) as PendingMutualTrustSummary[];
    set({ peers, inboundMutualTrust });
  },

  rejectMutualTrust: async (requestId: string) => {
    await invoke("reject_mutual_trust_request", { requestId });
    const inboundMutualTrust = (await invoke(
      "list_inbound_mutual_trust_requests"
    )) as PendingMutualTrustSummary[];
    set({ inboundMutualTrust });
  },

  connectTo: async (peerName: string) => {
    await invoke("connect_to_peer", { peerName });
  },

  remoteStatus: async (peerName: string) => {
    return await invoke("remote_peer_status", { peerName });
  },
}));
