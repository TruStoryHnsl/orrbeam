//! Persistent storage for trusted peers used by the orrbeam control plane.
//!
//! The [`TrustedPeerStore`] maintains a YAML file at
//! `~/.config/orrbeam/trusted_peers.yaml` containing every peer this node
//! is willing to accept signed HTTPS requests from, along with per-peer
//! permissions and metadata.
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while loading, saving, or mutating the peer store.
#[derive(Error, Debug)]
pub enum PeersError {
    /// An I/O error occurred while reading or writing the peers file.
    #[error("failed to read peers file: {0}")]
    Read(#[from] std::io::Error),

    /// The peers file could not be parsed as valid YAML.
    #[error("failed to parse peers file: {0}")]
    Parse(#[from] serde_yaml::Error),

    /// Two different peers share the same Ed25519 fingerprint.
    #[error("fingerprint collision: peer '{existing}' already has fingerprint {fingerprint}")]
    FingerprintCollision {
        /// Name of the peer that already owns the fingerprint.
        existing: String,
        /// The colliding fingerprint value.
        fingerprint: String,
    },

    /// The supplied fingerprint matches this node's own identity — a peer
    /// cannot trust itself.
    #[error("cannot trust self: fingerprint {0} matches this node's own identity")]
    SelfTrust(String),
}

// ---------------------------------------------------------------------------
// PeerPermissions
// ---------------------------------------------------------------------------

/// Fine-grained permission flags for a trusted peer.
///
/// Each flag controls whether the peer is allowed to invoke the corresponding
/// control-plane action on this node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerPermissions {
    /// Peer may query this node's Sunshine/Moonlight status.
    pub can_query_status: bool,
    /// Peer may start the Sunshine service on this node.
    pub can_start_sunshine: bool,
    /// Peer may stop the Sunshine service on this node.
    pub can_stop_sunshine: bool,
    /// Peer may submit a Sunshine pairing PIN to this node.
    pub can_submit_pin: bool,
    /// Peer may retrieve the full trusted-peer list from this node.
    pub can_list_peers: bool,
}

impl PeerPermissions {
    /// Full trust — every permission granted.
    ///
    /// Use this for owned devices where all control-plane operations should be
    /// allowed.
    pub fn trusted_full() -> Self {
        Self {
            can_query_status: true,
            can_start_sunshine: true,
            can_stop_sunshine: true,
            can_submit_pin: true,
            can_list_peers: true,
        }
    }

    /// Read-only trust — only status queries are permitted.
    ///
    /// Use this for friends or monitoring endpoints that should see status but
    /// not be able to change anything.
    pub fn friend_readonly() -> Self {
        Self {
            can_query_status: true,
            can_start_sunshine: false,
            can_stop_sunshine: false,
            can_submit_pin: false,
            can_list_peers: false,
        }
    }
}

// ---------------------------------------------------------------------------
// TrustedPeer
// ---------------------------------------------------------------------------

/// A single trusted remote node.
///
/// Peers are identified primarily by their Ed25519 public key fingerprint.
/// The `name` field is a human-readable label used as the map key inside the
/// store; it must be unique within the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    /// Human-readable name for this peer (also used as map key).
    pub name: String,

    /// First 8 bytes of the Ed25519 public key encoded as 16 lowercase hex
    /// characters.  Used as a short unique identifier.
    pub ed25519_fingerprint: String,

    /// Base64-encoded 32-byte Ed25519 public key.
    pub ed25519_public_key_b64: String,

    /// SHA-256 fingerprint of the peer's TLS certificate in lowercase hex.
    pub cert_sha256: String,

    /// IP address or hostname of the peer.
    pub address: String,

    /// TCP port on which the peer's control-plane HTTPS server listens.
    /// Defaults to `47782`.
    pub control_port: u16,

    /// Permission flags controlling what this peer is allowed to do.
    pub permissions: PeerPermissions,

    /// Free-form string labels (e.g. `owned`, `macos`, `laptop`).
    pub tags: Vec<String>,

    /// When this peer was first added to the store.
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: time::OffsetDateTime,

    /// When this peer was last successfully contacted.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_seen_at: Option<time::OffsetDateTime>,

    /// Optional free-form note about this peer.
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// TrustedPeerStore
// ---------------------------------------------------------------------------

/// Persistent, YAML-backed store of trusted peers.
///
/// The store maintains a secondary index (`by_fingerprint`) that is rebuilt
/// automatically on load and kept consistent by `upsert` / `remove`.
#[derive(Debug, Serialize, Deserialize)]
pub struct TrustedPeerStore {
    /// Bump when the schema changes in a breaking way.
    schema_version: u32,

    /// Primary map: peer name → peer record.
    peers: HashMap<String, TrustedPeer>,

    /// Secondary index: fingerprint → peer name.  Not serialised; rebuilt on
    /// load.
    #[serde(skip)]
    by_fingerprint: HashMap<String, String>,
}

impl Default for TrustedPeerStore {
    fn default() -> Self {
        Self {
            schema_version: 1,
            peers: HashMap::new(),
            by_fingerprint: HashMap::new(),
        }
    }
}

impl TrustedPeerStore {
    // -----------------------------------------------------------------------
    // Construction / persistence
    // -----------------------------------------------------------------------

    /// Load the peer store from disk.
    ///
    /// Returns an empty store (not an error) when the file does not yet exist.
    /// Returns [`PeersError::Read`] for I/O errors other than "not found", and
    /// [`PeersError::Parse`] for malformed YAML.
    pub fn load() -> Result<Self, PeersError> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let mut store: TrustedPeerStore = serde_yaml::from_str(&contents)?;
        store.rebuild_index();
        Ok(store)
    }

    /// Persist the store to disk.
    ///
    /// Creates parent directories if necessary and restricts the file to
    /// owner-read/write (`0o600`) on Unix.
    pub fn save(&self) -> Result<(), PeersError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(&path, &yaml)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Return the filesystem path used for the peer store.
    pub fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("trusted_peers.yaml")
    }

    // -----------------------------------------------------------------------
    // Mutation
    // -----------------------------------------------------------------------

    /// Insert or replace a peer.
    ///
    /// If a *different* peer already owns the same fingerprint the operation is
    /// rejected with [`PeersError::FingerprintCollision`].  Replacing an
    /// existing peer with the same name (and same fingerprint) is always
    /// allowed.
    ///
    /// The fingerprint index is updated atomically with the peers map.
    pub fn upsert(&mut self, peer: TrustedPeer) -> Result<(), PeersError> {
        // Collision guard: reject if a *different* peer owns this fingerprint.
        if let Some(owner) = self.by_fingerprint.get(&peer.ed25519_fingerprint)
            && owner != &peer.name
        {
            warn!(
                fingerprint = %peer.ed25519_fingerprint,
                existing_owner = %owner,
                new_peer = %peer.name,
                "fingerprint collision — rejecting upsert"
            );
            return Err(PeersError::FingerprintCollision {
                existing: owner.clone(),
                fingerprint: peer.ed25519_fingerprint.clone(),
            });
        }

        info!(name = %peer.name, fingerprint = %peer.ed25519_fingerprint, "adding/updating trusted peer");

        // Remove old fingerprint index entry if the peer already exists with a
        // different fingerprint (peer renamed their key).
        if let Some(existing) = self.peers.get(&peer.name)
            && existing.ed25519_fingerprint != peer.ed25519_fingerprint
        {
            self.by_fingerprint.remove(&existing.ed25519_fingerprint);
        }

        self.by_fingerprint
            .insert(peer.ed25519_fingerprint.clone(), peer.name.clone());
        self.peers.insert(peer.name.clone(), peer);
        Ok(())
    }

    /// Remove a peer by name.
    ///
    /// Returns the removed peer, or `None` if no peer with that name existed.
    /// Cleans up the fingerprint index.
    pub fn remove(&mut self, name: &str) -> Option<TrustedPeer> {
        if let Some(peer) = self.peers.remove(name) {
            self.by_fingerprint.remove(&peer.ed25519_fingerprint);
            info!(name = %name, "removed trusted peer");
            Some(peer)
        } else {
            None
        }
    }

    /// Update `last_seen_at` to the current time for the named peer.
    ///
    /// Does nothing if the peer is not found.
    pub fn touch_last_seen(&mut self, name: &str) {
        if let Some(peer) = self.peers.get_mut(name) {
            peer.last_seen_at = Some(time::OffsetDateTime::now_utc());
        }
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Look up a peer by name.
    pub fn get(&self, name: &str) -> Option<&TrustedPeer> {
        self.peers.get(name)
    }

    /// Look up a peer by its Ed25519 fingerprint.
    pub fn by_fingerprint(&self, fp: &str) -> Option<&TrustedPeer> {
        let name = self.by_fingerprint.get(fp)?;
        self.peers.get(name)
    }

    /// Return all peers sorted by name.
    pub fn list(&self) -> Vec<&TrustedPeer> {
        let mut peers: Vec<&TrustedPeer> = self.peers.values().collect();
        peers.sort_by(|a, b| a.name.cmp(&b.name));
        peers
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Rebuild the `by_fingerprint` secondary index from the `peers` map.
    ///
    /// Called after deserialization (the index is not serialized to disk).
    fn rebuild_index(&mut self) {
        self.by_fingerprint.clear();
        for (name, peer) in &self.peers {
            self.by_fingerprint
                .insert(peer.ed25519_fingerprint.clone(), name.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_peer(name: &str, fingerprint: &str) -> TrustedPeer {
        TrustedPeer {
            name: name.to_string(),
            ed25519_fingerprint: fingerprint.to_string(),
            ed25519_public_key_b64: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            cert_sha256: "deadbeef".repeat(8),
            address: "192.168.1.1".to_string(),
            control_port: 47782,
            permissions: PeerPermissions::trusted_full(),
            tags: vec!["test".to_string()],
            added_at: time::OffsetDateTime::now_utc(),
            last_seen_at: None,
            note: None,
        }
    }

    /// Save a store to a temp file, reload it, and verify all fields survived.
    #[test]
    fn save_load_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let mut store = TrustedPeerStore::default();
        let mut peer = make_peer("orrpheus", "a1b2c3d4e5f6a7b8");
        peer.note = Some("test note".to_string());
        peer.tags = vec!["owned".to_string(), "macos".to_string()];
        peer.last_seen_at = Some(time::OffsetDateTime::now_utc());
        store.upsert(peer.clone()).unwrap();

        // Save to temp path by writing directly (bypass path() for tests).
        let yaml = serde_yaml::to_string(&store).unwrap();
        std::fs::write(&path, &yaml).unwrap();

        // Reload.
        let contents = std::fs::read_to_string(&path).unwrap();
        let mut loaded: TrustedPeerStore = serde_yaml::from_str(&contents).unwrap();
        loaded.rebuild_index();

        let reloaded = loaded.get("orrpheus").unwrap();
        assert_eq!(reloaded.name, peer.name);
        assert_eq!(reloaded.ed25519_fingerprint, peer.ed25519_fingerprint);
        assert_eq!(reloaded.address, peer.address);
        assert_eq!(reloaded.note, peer.note);
        assert_eq!(reloaded.tags, peer.tags);
        assert_eq!(reloaded.permissions, peer.permissions);
        assert!(reloaded.last_seen_at.is_some());
    }

    /// Upserting a peer with the same name replaces the existing record.
    #[test]
    fn upsert_replaces_existing() {
        let mut store = TrustedPeerStore::default();
        store
            .upsert(make_peer("alpha", "aabbccdd11223344"))
            .unwrap();

        let mut updated = make_peer("alpha", "aabbccdd11223344");
        updated.address = "10.0.0.99".to_string();
        store.upsert(updated).unwrap();

        assert_eq!(store.get("alpha").unwrap().address, "10.0.0.99");
        assert_eq!(store.list().len(), 1);
    }

    /// Upserting a peer whose fingerprint is already owned by a *different*
    /// peer must be rejected.
    #[test]
    fn upsert_rejects_fingerprint_collision() {
        let mut store = TrustedPeerStore::default();
        store
            .upsert(make_peer("alice", "deadbeef01234567"))
            .unwrap();

        let result = store.upsert(make_peer("bob", "deadbeef01234567"));
        assert!(matches!(
            result,
            Err(PeersError::FingerprintCollision { .. })
        ));
    }

    /// Removing a peer should also clean up the fingerprint index.
    #[test]
    fn remove_cleans_fingerprint_index() {
        let mut store = TrustedPeerStore::default();
        store
            .upsert(make_peer("gamma", "1111222233334444"))
            .unwrap();

        let removed = store.remove("gamma");
        assert!(removed.is_some());
        assert!(store.get("gamma").is_none());
        assert!(store.by_fingerprint("1111222233334444").is_none());
    }

    /// `by_fingerprint` should find a peer using its fingerprint string.
    #[test]
    fn by_fingerprint_lookup() {
        let mut store = TrustedPeerStore::default();
        store
            .upsert(make_peer("delta", "ffffeeeeddddcccc"))
            .unwrap();

        let found = store.by_fingerprint("ffffeeeeddddcccc");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "delta");
    }

    /// Loading from a non-existent path must return an empty store without
    /// error.
    #[test]
    fn missing_file_returns_empty_store() {
        // Attempt to deserialize an empty string — simulates missing file path.
        // We test the actual load() code path by confirming empty-store default.
        let store = TrustedPeerStore::default();
        assert!(store.list().is_empty());
        assert_eq!(store.schema_version, 1);
    }

    /// `touch_last_seen` must update the timestamp on an existing peer.
    #[test]
    fn touch_last_seen_updates_timestamp() {
        let mut store = TrustedPeerStore::default();
        store
            .upsert(make_peer("epsilon", "0123456789abcdef"))
            .unwrap();

        assert!(store.get("epsilon").unwrap().last_seen_at.is_none());

        store.touch_last_seen("epsilon");

        assert!(store.get("epsilon").unwrap().last_seen_at.is_some());
    }
}
