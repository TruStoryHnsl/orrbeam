use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("failed to read identity: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid key data")]
    InvalidKey,
}

/// Ed25519 node identity for mesh authentication.
#[derive(Clone)]
pub struct Identity {
    signing_key: SigningKey,
}

/// Serializable public identity info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub fingerprint: String,
    pub public_key: Vec<u8>,
}

impl Identity {
    /// Load existing identity or generate a new one.
    pub fn load_or_create() -> Result<Self, IdentityError> {
        let path = Self::key_path();
        if path.exists() {
            let bytes = std::fs::read(&path)?;
            let key_bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidKey)?;
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
        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
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
