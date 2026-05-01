//! Remote peer management and connection orchestration commands.
//!
//! This module provides 11 Tauri commands for:
//! - Managing the local trusted-peer store (list, add, remove, update)
//! - TOFU mutual-trust negotiation (request, approve, reject, poll)
//! - Remote connection orchestration (`connect_to_peer` — the crown-jewel state machine)
//! - Debug helpers (`remote_peer_status`)

use crate::AppState;
use crate::error::AppError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use orrbeam_core::peers::{PeerPermissions, TrustedPeer};
use orrbeam_core::wire::HelloPayload;
use orrbeam_net::client::ControlClient;
use orrbeam_net::server::MutualTrustStatus;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared output types
// ---------------------------------------------------------------------------

/// Sanitized public view of a trusted peer (no raw signing keys).
#[derive(Debug, Serialize)]
pub struct PublicPeer {
    /// Human-readable name.
    pub name: String,
    /// Ed25519 key fingerprint (16 hex chars).
    pub ed25519_fingerprint: String,
    /// SHA-256 fingerprint of the peer's TLS certificate (hex).
    pub cert_sha256: String,
    /// IP address or hostname of the peer.
    pub address: String,
    /// TCP port for the peer's control-plane HTTPS server.
    pub control_port: u16,
    /// Permission flags.
    pub permissions: PeerPermissions,
    /// Free-form string labels.
    pub tags: Vec<String>,
    /// ISO 8601 timestamp when the peer was added.
    pub added_at: String,
    /// ISO 8601 timestamp of last successful contact, if any.
    pub last_seen_at: Option<String>,
    /// Optional note.
    pub note: Option<String>,
}

/// Summary of an inbound mutual-trust request waiting for local approval.
#[derive(Debug, Serialize)]
pub struct PendingMutualTrustSummary {
    /// UUID of the request.
    pub request_id: String,
    /// Public identity of the initiating node.
    pub initiator: HelloPayload,
    /// Optional note from the initiator.
    pub note: Option<String>,
    /// ISO 8601 timestamp when the request was received.
    pub created_at: String,
    /// Current status: `"pending"`, `"approved"`, or `"rejected"`.
    pub status: String,
}

/// Result of a `request_mutual_trust` call — returned to the frontend so it
/// can display the TOFU confirmation dialog.
#[derive(Debug, Serialize)]
pub struct MutualTrustInitResult {
    /// Client-generated UUID for this request (used to poll the receiver).
    pub request_id: String,
    /// Public identity of the receiver as fetched via bootstrap hello.
    pub receiver_hello: HelloPayload,
}

/// Payload emitted via the `peering:progress` Tauri event during `connect_to_peer`.
#[derive(Debug, Serialize, Clone)]
struct PeeringProgress {
    /// Current stage label (e.g. `"resolving"`, `"probing"`, `"done"`).
    pub stage: String,
    /// Human-readable name of the target peer.
    pub peer: String,
    /// Optional informational detail for the current stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// If set, a user-facing error description; the operation has failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Draft input type
// ---------------------------------------------------------------------------

/// Input for `confirm_trusted_peer` — all fields required to persist a peer
/// whose hello payload was already verified via the TOFU dialog.
#[derive(Debug, Deserialize)]
pub struct PeerDraft {
    /// Human-readable name the user chose for this peer.
    pub name: String,
    /// Ed25519 fingerprint from the peer's hello payload.
    pub ed25519_fingerprint: String,
    /// Full Ed25519 public key (base64 standard no-pad) from the peer's hello.
    pub ed25519_public_key_b64: String,
    /// TLS certificate SHA-256 fingerprint from the peer's hello payload.
    pub cert_sha256: String,
    /// IP address or hostname to connect to.
    pub address: String,
    /// Control-plane TCP port.
    pub control_port: u16,
    /// Optional tags (e.g. `["owned", "linux"]`).
    pub tags: Vec<String>,
    /// Optional free-form note.
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: build our own HelloPayload from AppState
// ---------------------------------------------------------------------------

async fn build_own_hello(state: &AppState) -> HelloPayload {
    let config = state.config.read().await;
    HelloPayload {
        node_name: config.node_name.clone(),
        ed25519_fingerprint: state.identity.fingerprint(),
        ed25519_public_key_b64: STANDARD_NO_PAD.encode(state.identity.public_key().as_bytes()),
        cert_sha256: state.tls.cert_sha256_hex.clone(),
        control_port: config.api_port,
        sunshine_available: false, // best-effort; not critical for trust exchange
        moonlight_available: false,
        os: state.platform.info().os,
        version: orrbeam_core::wire::PROTOCOL_VERSION.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Helper: emit a peering:progress event (fire-and-forget, log on failure)
// ---------------------------------------------------------------------------

fn emit_progress(
    app: &AppHandle,
    stage: &str,
    peer: &str,
    detail: Option<&str>,
    error: Option<&str>,
) {
    let payload = PeeringProgress {
        stage: stage.to_string(),
        peer: peer.to_string(),
        detail: detail.map(str::to_string),
        error: error.map(str::to_string),
    };
    if let Err(e) = app.emit("peering:progress", payload) {
        tracing::warn!(stage, peer, "failed to emit peering:progress event: {e}");
    }
}

// ---------------------------------------------------------------------------
// 1. list_trusted_peers
// ---------------------------------------------------------------------------

/// Return all trusted peers as sanitized `PublicPeer` records (no raw keys).
#[tauri::command]
pub async fn list_trusted_peers(state: State<'_, AppState>) -> Result<Vec<PublicPeer>, String> {
    let store = state.peers.read().await;
    let peers = store
        .list()
        .into_iter()
        .map(|p| PublicPeer {
            name: p.name.clone(),
            ed25519_fingerprint: p.ed25519_fingerprint.clone(),
            cert_sha256: p.cert_sha256.clone(),
            address: p.address.clone(),
            control_port: p.control_port,
            permissions: p.permissions.clone(),
            tags: p.tags.clone(),
            added_at: p
                .added_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| p.added_at.unix_timestamp().to_string()),
            last_seen_at: p.last_seen_at.and_then(|ts| {
                ts.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            }),
            note: p.note.clone(),
        })
        .collect();
    Ok(peers)
}

// ---------------------------------------------------------------------------
// 2. fetch_peer_hello
// ---------------------------------------------------------------------------

/// Fetch the public identity of an unknown peer via the TOFU bootstrap hello.
///
/// Uses `danger_accept_invalid_certs` — the caller must pin the returned
/// `cert_sha256` immediately after displaying the TOFU confirmation dialog.
#[tauri::command]
pub async fn fetch_peer_hello(address: String, port: u16) -> Result<HelloPayload, AppError> {
    ControlClient::bootstrap_hello(&address, port)
        .await
        .map_err(|e| AppError::Network(format!("Could not reach {address}:{port} — {e}")))
}

// ---------------------------------------------------------------------------
// 3. confirm_trusted_peer
// ---------------------------------------------------------------------------

/// Persist a peer that the user approved in the TOFU dialog.
///
/// Builds a `TrustedPeer` from `draft` with full trust permissions and upserts
/// it into the trusted-peer store, then saves to disk.
#[tauri::command]
pub async fn confirm_trusted_peer(
    state: State<'_, AppState>,
    draft: PeerDraft,
) -> Result<(), AppError> {
    let peer = TrustedPeer {
        name: draft.name.clone(),
        ed25519_fingerprint: draft.ed25519_fingerprint,
        ed25519_public_key_b64: draft.ed25519_public_key_b64,
        cert_sha256: draft.cert_sha256,
        address: draft.address,
        control_port: draft.control_port,
        permissions: PeerPermissions::trusted_full(),
        tags: draft.tags,
        added_at: time::OffsetDateTime::now_utc(),
        last_seen_at: None,
        note: draft.note,
    };

    let mut store = state.peers.write().await;
    store
        .upsert(peer)
        .map_err(|e| AppError::Internal(format!("Failed to add peer '{}': {e}", draft.name)))?;
    store
        .save()
        .map_err(|e| AppError::Internal(format!("Failed to save peer store: {e}")))?;

    tracing::info!(peer = %draft.name, "confirmed and persisted trusted peer");
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. remove_trusted_peer
// ---------------------------------------------------------------------------

/// Remove a trusted peer by name.
///
/// Returns `true` if the peer existed and was removed, `false` if not found.
#[tauri::command]
pub async fn remove_trusted_peer(
    state: State<'_, AppState>,
    name: String,
) -> Result<bool, AppError> {
    let mut store = state.peers.write().await;
    let removed = store.remove(&name).is_some();
    if removed {
        store.save().map_err(|e| {
            AppError::Internal(format!(
                "Failed to save peer store after removing '{name}': {e}"
            ))
        })?;
        tracing::info!(peer = %name, "removed trusted peer");
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------
// 5. update_peer_permissions
// ---------------------------------------------------------------------------

/// Update the permission flags for an existing trusted peer.
#[tauri::command]
pub async fn update_peer_permissions(
    state: State<'_, AppState>,
    name: String,
    permissions: PeerPermissions,
) -> Result<(), AppError> {
    let mut store = state.peers.write().await;

    // Clone the existing peer, update permissions, upsert.
    let mut peer = store
        .get(&name)
        .ok_or_else(|| AppError::NotFound(format!("peer \'{name}\' not found")))?
        .clone();

    peer.permissions = permissions;

    store.upsert(peer).map_err(|e| {
        AppError::Internal(format!("Failed to update permissions for '{name}': {e}"))
    })?;
    store
        .save()
        .map_err(|e| AppError::Internal(format!("Failed to save peer store: {e}")))?;

    tracing::info!(peer = %name, "updated peer permissions");
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. request_mutual_trust
// ---------------------------------------------------------------------------

/// Initiate a mutual-trust TOFU request to a remote node.
///
/// Steps:
/// 1. Fetch the receiver's hello via bootstrap (TOFU, no auth).
/// 2. POST our own hello + a fresh UUID to the receiver's
///    `POST /v1/mutual-trust-request`.
/// 3. Spawn a background task that polls
///    `GET /v1/mutual-trust-request/{id}` every 2 s until approved, rejected,
///    or the 60 s timeout elapses.  On approval the receiver is automatically
///    persisted as a trusted peer.
/// 4. Return `{ request_id, receiver_hello }` so the UI can display the TOFU
///    confirmation panel.
#[tauri::command]
pub async fn request_mutual_trust(
    state: State<'_, AppState>,
    address: String,
    port: u16,
    note: Option<String>,
) -> Result<MutualTrustInitResult, AppError> {
    // Step 1: fetch receiver's hello.
    let receiver_hello = ControlClient::bootstrap_hello(&address, port)
        .await
        .map_err(|e| AppError::Network(format!("Could not reach {address}:{port} — {e}")))?;

    // Step 2: build our own hello and POST the trust request.
    let our_hello = build_own_hello(&state).await;
    let request_id = Uuid::new_v4();

    // Build a one-shot HTTP client (no cert pinning — this is a TOFU request).
    let http: reqwest::Client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;

    let url = format!("https://{address}:{port}/v1/mutual-trust-request");
    let body = serde_json::json!({
        "initiator": our_hello,
        "note": note,
        "request_id": request_id,
    });

    let resp = http.post(&url).json(&body).send().await.map_err(|e| {
        AppError::Network(format!(
            "Failed to send trust request to {address}:{port} — {e}"
        ))
    })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        return Err(AppError::Internal(format!(
            "Receiver rejected the trust request (HTTP {status})"
        )));
    }

    tracing::info!(
        request_id = %request_id,
        address = %address,
        port,
        "mutual trust request sent — waiting for approval"
    );

    // Step 3: spawn background poller.
    let peers_arc = state.peers.clone();
    let receiver_hello_clone = receiver_hello.clone();
    let address_clone = address.clone();

    tokio::spawn(async move {
        use std::time::Duration;

        // Build a new TOFU client for polling (no cert pin for this call set).
        let poll_client: reqwest::Client = match reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("mutual trust poller: failed to build HTTP client: {e}");
                return;
            }
        };

        let poll_url =
            format!("https://{address_clone}:{port}/v1/mutual-trust-request/{request_id}");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);

        loop {
            if tokio::time::Instant::now() >= deadline {
                tracing::warn!(request_id = %request_id, "mutual trust request timed out");
                break;
            }

            tokio::time::sleep(Duration::from_secs(2)).await;

            let result = poll_client.get(&poll_url).send().await;
            match result {
                Err(e) => {
                    tracing::debug!(request_id = %request_id, "poll failed: {e}");
                    continue;
                }
                Ok(r) => {
                    if !r.status().is_success() {
                        tracing::debug!(
                            request_id = %request_id,
                            status = r.status().as_u16(),
                            "poll returned non-success; request may have expired"
                        );
                        break;
                    }

                    let poll_body: serde_json::Value = match r.json().await {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::debug!(request_id = %request_id, "poll body parse error: {e}");
                            continue;
                        }
                    };

                    let status_str = poll_body
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match status_str {
                        "pending" => continue,
                        "approved" => {
                            tracing::info!(
                                request_id = %request_id,
                                "mutual trust approved — persisting receiver as trusted peer"
                            );
                            // Build TrustedPeer from receiver_hello.
                            let peer = TrustedPeer {
                                name: receiver_hello_clone.node_name.clone(),
                                ed25519_fingerprint: receiver_hello_clone
                                    .ed25519_fingerprint
                                    .clone(),
                                ed25519_public_key_b64: receiver_hello_clone
                                    .ed25519_public_key_b64
                                    .clone(),
                                cert_sha256: receiver_hello_clone.cert_sha256.clone(),
                                address: address_clone.clone(),
                                control_port: receiver_hello_clone.control_port,
                                permissions: PeerPermissions::trusted_full(),
                                tags: vec![],
                                added_at: time::OffsetDateTime::now_utc(),
                                last_seen_at: None,
                                note: None,
                            };

                            let mut store = peers_arc.write().await;
                            if let Err(e) = store.upsert(peer) {
                                tracing::error!(request_id = %request_id, "failed to persist peer: {e}");
                            } else if let Err(e) = store.save() {
                                tracing::error!(request_id = %request_id, "failed to save peer store: {e}");
                            } else {
                                tracing::info!(
                                    peer = %receiver_hello_clone.node_name,
                                    "trusted peer persisted after mutual trust approval"
                                );
                            }
                            break;
                        }
                        "rejected" => {
                            tracing::info!(
                                request_id = %request_id,
                                "mutual trust request was rejected by the receiver"
                            );
                            break;
                        }
                        other => {
                            tracing::warn!(request_id = %request_id, status = other, "unexpected poll status");
                            continue;
                        }
                    }
                }
            }
        }
    });

    Ok(MutualTrustInitResult {
        request_id: request_id.to_string(),
        receiver_hello,
    })
}

// ---------------------------------------------------------------------------
// 7. approve_mutual_trust_request
// ---------------------------------------------------------------------------

/// Approve an inbound mutual-trust request (receiver side).
///
/// Looks up `request_id` in the pending map held by the control server,
/// builds a `TrustedPeer` from the initiator's hello, persists it, then
/// transitions the pending entry to `Approved` so the initiator's poller sees
/// the outcome.
#[tauri::command]
pub async fn approve_mutual_trust_request(
    state: State<'_, AppState>,
    _app: AppHandle,
    request_id: String,
) -> Result<(), AppError> {
    let id: Uuid = request_id
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("invalid request ID: {request_id}")))?;

    // Fetch the pending entry (read lock).
    let initiator_hello = {
        let map = state.pending_mutual_trust.read().await;
        let entry = map.get(&id).ok_or_else(|| {
            AppError::NotFound(format!(
                "trust request '{request_id}' not found or already resolved"
            ))
        })?;

        if entry.status != MutualTrustStatus::Pending {
            return Err(AppError::Internal(format!(
                "Trust request '{request_id}' is already {:?}",
                entry.status
            )));
        }
        entry.initiator.clone()
    };

    // Persist the initiator as a trusted peer.
    let peer = TrustedPeer {
        name: initiator_hello.node_name.clone(),
        ed25519_fingerprint: initiator_hello.ed25519_fingerprint.clone(),
        ed25519_public_key_b64: initiator_hello.ed25519_public_key_b64.clone(),
        cert_sha256: initiator_hello.cert_sha256.clone(),
        address: initiator_hello.node_name.clone(), // best we have without an address field
        control_port: initiator_hello.control_port,
        permissions: PeerPermissions::trusted_full(),
        tags: vec![],
        added_at: time::OffsetDateTime::now_utc(),
        last_seen_at: None,
        note: None,
    };

    {
        let mut store = state.peers.write().await;
        store
            .upsert(peer)
            .map_err(|e| AppError::Internal(format!("Failed to persist initiator as peer: {e}")))?;
        store
            .save()
            .map_err(|e| AppError::Internal(format!("Failed to save peer store: {e}")))?;
    }

    // Build our own hello to embed in the approval response.
    let our_hello = build_own_hello(&state).await;

    // Transition the pending entry to Approved.
    {
        let mut map = state.pending_mutual_trust.write().await;
        if let Some(entry) = map.get_mut(&id) {
            entry.status = MutualTrustStatus::Approved;
            entry.receiver_hello = Some(our_hello);
        }
    }

    tracing::info!(
        request_id = %request_id,
        initiator = %initiator_hello.node_name,
        "mutual trust request approved"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. reject_mutual_trust_request
// ---------------------------------------------------------------------------

/// Reject an inbound mutual-trust request (receiver side).
#[tauri::command]
pub async fn reject_mutual_trust_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<(), AppError> {
    let id: Uuid = request_id
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("invalid request ID: {request_id}")))?;

    let mut map = state.pending_mutual_trust.write().await;
    let entry = map.get_mut(&id).ok_or_else(|| {
        AppError::NotFound(format!(
            "trust request '{request_id}' not found or already resolved"
        ))
    })?;

    if entry.status != MutualTrustStatus::Pending {
        return Err(AppError::InvalidInput(format!(
            "trust request '{request_id}' is already {:?}",
            entry.status
        )));
    }

    entry.status = MutualTrustStatus::Rejected;

    tracing::info!(request_id = %request_id, "mutual trust request rejected");
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. list_inbound_mutual_trust_requests
// ---------------------------------------------------------------------------

/// List all inbound mutual-trust requests (pending, approved, and rejected).
#[tauri::command]
pub async fn list_inbound_mutual_trust_requests(
    state: State<'_, AppState>,
) -> Result<Vec<PendingMutualTrustSummary>, String> {
    let map = state.pending_mutual_trust.read().await;

    let mut summaries: Vec<PendingMutualTrustSummary> = map
        .iter()
        .map(|(id, entry)| PendingMutualTrustSummary {
            request_id: id.to_string(),
            initiator: entry.initiator.clone(),
            note: entry.note.clone(),
            created_at: entry
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| entry.created_at.unix_timestamp().to_string()),
            status: match entry.status {
                MutualTrustStatus::Pending => "pending".to_string(),
                MutualTrustStatus::Approved => "approved".to_string(),
                MutualTrustStatus::Rejected => "rejected".to_string(),
            },
        })
        .collect();

    // Sort by created_at descending (newest first).
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(summaries)
}

// ---------------------------------------------------------------------------
// 10. connect_to_peer — the main state machine
// ---------------------------------------------------------------------------

/// Orchestrate a full remote-desktop connection to a trusted peer.
///
/// State machine stages (each emits a `peering:progress` Tauri event):
///
/// 1. `resolving`       — look up peer by name in the trusted-peer store
/// 2. `probing`         — call `client.status()` to confirm orrbeam is reachable
/// 3. `remote_starting` — start Sunshine on the remote if not already running;
///                         polls until Running (20 s timeout)
/// 4. `pin_generating`  — generate a 4-digit random PIN
/// 5. `paired_parallel` — concurrently:
///                         (a) `platform.pair_moonlight(...)` — local Moonlight pairing
///                         (b) `client.pair_accept(pin, our_node_name)` — remote PIN submit
/// 6. `streaming_local` — launch `platform.start_moonlight(...)` to begin streaming
/// 7. `done`            — success event emitted
///
/// On any error: emits `peering:progress` with `error` set and returns `Err`.
/// Remote Sunshine is **not** stopped on failure.
#[tauri::command]
pub async fn connect_to_peer(
    state: State<'_, AppState>,
    app: AppHandle,
    peer_name: String,
) -> Result<(), AppError> {
    // ── Stage: resolving ────────────────────────────────────────────────────
    emit_progress(&app, "resolving", &peer_name, None, None);

    let peer = {
        let store = state.peers.read().await;
        store
            .get(&peer_name)
            .ok_or_else(|| {
                let msg_str = format!("Peer '{peer_name}' not found in trusted peers");
                emit_progress(&app, "resolving", &peer_name, None, Some(&msg_str));
                AppError::Network(msg_str)
            })?
            .clone()
    };

    let client = ControlClient::new(state.identity.clone(), &peer).map_err(|e| {
        let msg_str = format!("Failed to build connection client for '{peer_name}': {e}");
        emit_progress(&app, "resolving", &peer_name, None, Some(&msg_str));
        AppError::Network(msg_str)
    })?;

    // ── Stage: probing ──────────────────────────────────────────────────────
    emit_progress(&app, "probing", &peer_name, None, None);

    let status_resp = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.status(),
    )
    .await
    .map_err(|_| {
        let msg_str = format!(
            "orrbeam is not running on '{peer_name}'. Start it manually, then retry."
        );
        emit_progress(&app, "probing", &peer_name, None, Some(&msg_str));
                AppError::Network(msg_str)
            })?
    .map_err(|e| {
        let msg_str = format!(
            "orrbeam is not running on '{peer_name}'. Start it manually, then retry. (detail: {e})"
        );
        emit_progress(&app, "probing", &peer_name, None, Some(&msg_str));
                AppError::Network(msg_str)
            })?;

    // ── Stage: remote_starting ──────────────────────────────────────────────
    emit_progress(&app, "remote_starting", &peer_name, None, None);

    let sunshine_running = status_resp
        .sunshine
        .get("running")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !sunshine_running {
        // Ask remote to start Sunshine.
        client.sunshine_start().await.map_err(|e| {
            let msg_str = format!("Failed to start Sunshine on '{peer_name}': {e}");
            emit_progress(&app, "remote_starting", &peer_name, None, Some(&msg_str));
            AppError::Network(msg_str)
        })?;

        // Poll until running (20 s timeout).
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            if tokio::time::Instant::now() >= deadline {
                let msg_str = format!("Sunshine on '{peer_name}' did not start within 20 seconds");
                emit_progress(&app, "remote_starting", &peer_name, None, Some(&msg_str));
                return Err(AppError::Network(msg_str));
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            match client.status().await {
                Ok(s) => {
                    let running = s
                        .sunshine
                        .get("running")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if running {
                        break;
                    }
                    emit_progress(
                        &app,
                        "remote_starting",
                        &peer_name,
                        Some("waiting for Sunshine to start…"),
                        None,
                    );
                }
                Err(e) => {
                    tracing::debug!(peer = %peer_name, "status poll during remote_starting: {e}");
                }
            }
        }
    }

    // ── Stage: pin_generating ───────────────────────────────────────────────
    emit_progress(&app, "pin_generating", &peer_name, None, None);

    // 4-digit zero-padded PIN — never logged.
    let pin = format!("{:04}", rand::rng().random_range(0..10000u32));

    let our_node_name = {
        let cfg = state.config.read().await;
        cfg.node_name.clone()
    };

    // ── Stage: paired_parallel ──────────────────────────────────────────────
    emit_progress(&app, "paired_parallel", &peer_name, None, None);

    let peer_address = peer.address.clone();
    let platform = state.platform.clone();
    let config_snap = {
        let cfg = state.config.read().await;
        cfg.clone()
    };

    // (a) Local Moonlight pairing — runs in a blocking thread since it spawns
    //     a child process.
    let pin_a = pin.clone();
    let addr_a = peer_address.clone();
    let platform_a = platform.clone();
    let config_a = config_snap.clone();
    let pair_local =
        tokio::task::spawn_blocking(move || platform_a.pair_moonlight(&config_a, &addr_a, &pin_a));

    // (b) Remote PIN submission to the peer's Sunshine via the control client.
    let pin_b = pin.clone();
    let name_b = our_node_name.clone();
    let pair_remote = client.pair_accept(&pin_b, &name_b);

    // Run both with a 25 s combined timeout.
    // `tokio::join!` produces a tuple (not a future), so wrap it in an async
    // block so `tokio::time::timeout` can race against it.
    let (local_result, remote_result) =
        tokio::time::timeout(std::time::Duration::from_secs(25), async {
            tokio::join!(pair_local, pair_remote)
        })
        .await
        .map_err(|_| {
            let msg_str = format!("Pairing with '{peer_name}' timed out after 25 seconds");
            emit_progress(&app, "paired_parallel", &peer_name, None, Some(&msg_str));
            AppError::Network(msg_str)
        })?;

    // Evaluate local result (spawn_blocking JoinHandle).
    local_result
        .map_err(|e| {
            let msg_str = format!("Pairing task panicked for '{peer_name}': {e}");
            emit_progress(&app, "paired_parallel", &peer_name, None, Some(&msg_str));
            AppError::Network(msg_str)
        })?
        .map_err(|e| {
            let msg_str = format!("Local Moonlight pairing failed for '{peer_name}': {e}");
            emit_progress(&app, "paired_parallel", &peer_name, None, Some(&msg_str));
            AppError::Network(msg_str)
        })?;

    // Evaluate remote result — `accepted: true` is the success signal.
    let pair_accept_resp = remote_result.map_err(|e| {
        let msg_str = format!("Remote pairing PIN was not accepted by '{peer_name}': {e}");
        emit_progress(&app, "paired_parallel", &peer_name, None, Some(&msg_str));
        AppError::Network(msg_str)
    })?;

    if !pair_accept_resp.accepted {
        let msg_str = format!("Sunshine on '{peer_name}' rejected the pairing PIN");
        emit_progress(&app, "paired_parallel", &peer_name, None, Some(&msg_str));
        return Err(AppError::Network(msg_str));
    }

    tracing::info!(peer = %peer_name, "pairing completed successfully (PIN <redacted>)");

    // ── Stage: streaming_local ──────────────────────────────────────────────
    emit_progress(&app, "streaming_local", &peer_name, None, None);

    let addr_stream = peer_address.clone();
    let platform_stream = platform.clone();
    let config_stream = config_snap.clone();

    tokio::task::spawn_blocking(move || {
        platform_stream.start_moonlight(&config_stream, &addr_stream, "Desktop", false, None)
    })
    .await
    .map_err(|e| {
        let msg_str = format!("Moonlight launch task panicked for '{peer_name}': {e}");
        emit_progress(&app, "streaming_local", &peer_name, None, Some(&msg_str));
        AppError::Network(msg_str)
    })?
    .map_err(|e| {
        let msg_str = format!("Failed to start Moonlight for '{peer_name}': {e}");
        emit_progress(&app, "streaming_local", &peer_name, None, Some(&msg_str));
        AppError::Network(msg_str)
    })?;

    // ── Stage: done ─────────────────────────────────────────────────────────
    emit_progress(&app, "done", &peer_name, Some("Connected"), None);
    tracing::info!(peer = %peer_name, "connect_to_peer: streaming session started");

    Ok(())
}

// ---------------------------------------------------------------------------
// 11. remote_peer_status — debug helper
// ---------------------------------------------------------------------------

/// Query the raw Sunshine/Moonlight status of a trusted peer.
///
/// Returns the JSON value as-is from the control client's `status()` call.
/// Intended for debugging and diagnostics.
#[tauri::command]
pub async fn remote_peer_status(
    state: State<'_, AppState>,
    peer_name: String,
) -> Result<serde_json::Value, AppError> {
    let peer = {
        let store = state.peers.read().await;
        store
            .get(&peer_name)
            .ok_or_else(|| AppError::NotFound(format!("peer \'{peer_name}\' not found")))?
            .clone()
    };

    let client = ControlClient::new(state.identity.clone(), &peer).map_err(|e| {
        AppError::Internal(format!("Failed to build client for '{peer_name}': {e}"))
    })?;

    let status = client
        .status()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query status of '{peer_name}': {e}")))?;

    Ok(serde_json::json!({
        "sunshine": status.sunshine,
        "moonlight": status.moonlight,
    }))
}
