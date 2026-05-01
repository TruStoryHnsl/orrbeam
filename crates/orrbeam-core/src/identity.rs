//! Ed25519 node identity for mesh authentication.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or generating an [`Identity`].
#[derive(Error, Debug)]
pub enum IdentityError {
    /// An I/O error while reading or writing the signing key file.
    #[error("failed to read identity: {0}")]
    Io(#[from] std::io::Error),
    /// The stored key bytes could not be decoded as a valid Ed25519 signing key.
    #[error("invalid key data")]
    InvalidKey,
}

/// Ed25519 node identity for mesh authentication.
#[derive(Clone)]
pub struct Identity {
    signing_key: SigningKey,
}

/// Serializable public identity info for sharing over the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicIdentity {
    /// Human-readable fingerprint (first 16 hex chars of the Ed25519 public key).
    pub fingerprint: String,
    /// Raw 32-byte Ed25519 public key bytes.
    pub public_key: Vec<u8>,
}

impl Identity {
    /// Load existing identity or generate a new one.
    pub fn load_or_create() -> Result<Self, IdentityError> {
        let path = Self::key_path();
        if path.exists() {
            let bytes = std::fs::read(&path)?;
            let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| IdentityError::InvalidKey)?;
            Ok(Self {
                signing_key: SigningKey::from_bytes(&key_bytes),
            })
        } else {
            let identity = Self::generate()?;
            identity.save()?;
            Ok(identity)
        }
    }

    /// Generate a new random identity.
    pub fn generate() -> Result<Self, IdentityError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Self { signing_key })
    }

    /// Save the signing key to disk.
    fn save(&self) -> Result<(), IdentityError> {
        let path = Self::key_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, self.signing_key.to_bytes())?;
        // Restrict file to owner only (chmod 0o600 on Unix; icacls on Windows).
        crate::secure_file::restrict_to_owner(&path)?;
        Ok(())
    }

    /// Get the public verifying key.
    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Human-readable fingerprint (first 16 hex chars of public key).
    pub fn fingerprint(&self) -> String {
        let pk = self.public_key();
        hex::encode(&pk.as_bytes()[..8])
    }

    /// Get serializable public identity.
    pub fn public_identity(&self) -> PublicIdentity {
        PublicIdentity {
            fingerprint: self.fingerprint(),
            public_key: self.public_key().as_bytes().to_vec(),
        }
    }

    /// Expose the signing key for TLS certificate derivation.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    fn key_path() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("identity").join("signing.key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_unique_keys() {
        let a = Identity::generate().expect("generate a");
        let b = Identity::generate().expect("generate b");
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn fingerprint_is_16_hex_chars() {
        let id = Identity::generate().expect("generate");
        let fp = id.fingerprint();
        assert_eq!(fp.len(), 16, "fingerprint must be 16 hex chars");
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn public_identity_matches_fingerprint() {
        let id = Identity::generate().expect("generate");
        let pub_id = id.public_identity();
        assert_eq!(pub_id.fingerprint, id.fingerprint());
        assert_eq!(pub_id.public_key.len(), 32);
    }

    #[test]
    fn signing_key_roundtrip() {
        let id = Identity::generate().expect("generate");
        let bytes = id.signing_key().to_bytes();
        let id2 = Identity {
            signing_key: SigningKey::from_bytes(&bytes),
        };
        assert_eq!(id.fingerprint(), id2.fingerprint());
    }
}
