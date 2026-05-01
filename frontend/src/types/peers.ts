/** Per-peer permission flags matching Rust PeerPermissions. */
export interface PeerPermissions {
  can_query_status: boolean;
  can_start_sunshine: boolean;
  can_stop_sunshine: boolean;
  can_submit_pin: boolean;
  can_list_peers: boolean;
}

/** A trusted peer as returned by list_trusted_peers. */
export interface TrustedPeer {
  name: string;
  ed25519_fingerprint: string;
  ed25519_public_key_b64: string;
  cert_sha256: string;
  address: string;
  control_port: number;
  permissions: PeerPermissions;
  tags: string[];
  added_at: string; // RFC 3339
  last_seen_at: string | null;
  note: string | null;
}

/** Hello payload from a remote node (TOFU response). */
export interface HelloPayload {
  node_name: string;
  ed25519_fingerprint: string;
  ed25519_public_key_b64: string;
  cert_sha256: string;
  control_port: number;
  sunshine_available: boolean;
  moonlight_available: boolean;
  os: string;
  version: string;
}

/** Draft for adding a new peer (confirm_trusted_peer input). */
export interface PeerDraft {
  name: string;
  ed25519_fingerprint: string;
  ed25519_public_key_b64: string;
  cert_sha256: string;
  address: string;
  control_port: number;
  tags: string[];
  note: string | null;
}

/** Result from request_mutual_trust. */
export interface MutualTrustInitResult {
  request_id: string;
  receiver_hello: HelloPayload;
}

/** Summary of a pending inbound mutual trust request. */
export interface PendingMutualTrustSummary {
  request_id: string;
  initiator_name: string;
  initiator_fingerprint: string;
  note: string | null;
  created_at: string;
}

/** Peering progress event payload. */
export interface PeeringProgress {
  stage: string;
  peer: string;
  detail: string | null;
  error: string | null;
}

/** TLS fingerprint response. */
export interface TlsFingerprint {
  cert_sha256: string;
  ed25519_fingerprint: string;
  control_port: number;
}
