//! TLS identity for the orrbeam control plane.
//!
//! Generates and manages a self-signed X.509 certificate derived from the node's
//! Ed25519 identity key.  The certificate is used by the control plane HTTPS server
//! (port 47782) and is bound to the node's cryptographic identity so that the
//! certificate fingerprint uniquely identifies the node in the mesh.
//!
//! # Storage paths
//! - Certificate: `~/.local/share/orrbeam/identity/control.cert.pem`
//! - Private key : `~/.local/share/orrbeam/identity/control.key.pem`
#![warn(missing_docs)]

use crate::identity::Identity;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info};

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur when creating or loading a [`TlsIdentity`].
#[derive(Debug, Error)]
pub enum TlsError {
    /// An I/O error while reading or writing certificate / key files.
    #[error("TLS I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// `rcgen` failed to generate or serialise the certificate.
    #[error("certificate generation error: {0}")]
    Rcgen(#[from] rcgen::Error),

    /// `rustls` rejected the certificate or key material.
    #[error("rustls error: {0}")]
    Rustls(#[from] rustls::Error),

    /// The key material could not be interpreted or converted.
    #[error("invalid key: {0}")]
    InvalidKey(String),
}

// ── TlsIdentity ───────────────────────────────────────────────────────────────

/// A self-signed TLS certificate derived from the node's Ed25519 identity.
///
/// Holds the certificate and private key in PEM format (for writing to disk / feeding
/// to rustls) and the raw DER bytes plus SHA-256 fingerprint (for peer verification).
#[derive(Clone)]
pub struct TlsIdentity {
    /// PEM-encoded self-signed certificate.
    pub cert_pem: String,

    /// PEM-encoded Ed25519 private key (PKCS#8).
    pub key_pem: String,

    /// DER-encoded certificate bytes.
    pub cert_der: Vec<u8>,

    /// Lowercase hex SHA-256 digest of `cert_der`, e.g.
    /// `"a3f2…"` (64 chars).  Used for out-of-band peer verification.
    pub cert_sha256_hex: String,
}

impl TlsIdentity {
    // ── Public API ────────────────────────────────────────────────────────────

    /// Load an existing TLS certificate from disk or generate a new one.
    ///
    /// The certificate is tied to `identity` — the Ed25519 signing key is used
    /// directly so that the certificate fingerprint changes only when the node
    /// identity changes.
    ///
    /// # Arguments
    /// * `identity`  – Node identity (provides the Ed25519 signing key).
    /// * `node_name` – Human-readable name embedded as the certificate CN and
    ///   used to build the SAN `<node_name>.orrbeam.local`.
    pub fn load_or_create(identity: &Identity, node_name: &str) -> Result<Self, TlsError> {
        let cert_path = Self::cert_path();
        let key_path = Self::key_path();

        if cert_path.exists() && key_path.exists() {
            debug!(
                cert = %cert_path.display(),
                "loading existing TLS identity from disk"
            );
            Self::load_from_disk(&cert_path, &key_path)
        } else {
            info!(
                node_name,
                "generating new TLS identity for control plane"
            );
            Self::generate_and_save(identity, node_name, &cert_path, &key_path)
        }
    }

    /// Build a rustls [`ServerConfig`] from this identity.
    ///
    /// Only TLS 1.3 is enabled.  The certificate and private key are loaded from
    /// the in-memory PEM strings held by this struct.
    pub fn rustls_server_config(&self) -> Result<ServerConfig, TlsError> {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls_pemfile::{certs, private_key};
        use std::io::BufReader;

        // Parse the certificate chain.
        let mut cert_reader = BufReader::new(self.cert_pem.as_bytes());
        let cert_chain: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()?;

        // Parse the private key.
        let mut key_reader = BufReader::new(self.key_pem.as_bytes());
        let private_key: PrivateKeyDer<'static> = private_key(&mut key_reader)?
            .ok_or_else(|| TlsError::InvalidKey("no private key found in PEM".into()))?;

        let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)?;

        Ok(config)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Read cert + key PEM files from `cert_path` / `key_path` and build a
    /// [`TlsIdentity`] from them.
    fn load_from_disk(cert_path: &PathBuf, key_path: &PathBuf) -> Result<Self, TlsError> {
        let cert_pem = std::fs::read_to_string(cert_path)?;
        let key_pem = std::fs::read_to_string(key_path)?;

        // Decode the DER bytes from the PEM to compute the fingerprint.
        let cert_der = Self::der_from_cert_pem(&cert_pem)?;
        let cert_sha256_hex = Self::sha256_hex(&cert_der);

        Ok(Self {
            cert_pem,
            key_pem,
            cert_der,
            cert_sha256_hex,
        })
    }

    /// Generate a new self-signed certificate tied to `identity` and persist it.
    fn generate_and_save(
        identity: &Identity,
        node_name: &str,
        cert_path: &PathBuf,
        key_path: &PathBuf,
    ) -> Result<Self, TlsError> {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        use rustls::pki_types::PrivatePkcs8KeyDer;
        use rcgen::PKCS_ED25519;

        // Convert the dalek SigningKey to PKCS#8 DER.
        let pkcs8_doc = identity
            .signing_key()
            .to_pkcs8_der()
            .map_err(|e| TlsError::InvalidKey(e.to_string()))?;

        // Wrap in an rcgen KeyPair via the PKCS#8 path.
        let pkcs8_key = PrivatePkcs8KeyDer::from(pkcs8_doc.as_bytes().to_vec());
        let key_pair = KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8_key, &PKCS_ED25519)?;

        // Build certificate parameters.
        let mut params = CertificateParams::default();

        // Distinguished name: CN=<node_name>, O=orrbeam, OU=<ed25519_fingerprint>
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, node_name);
        dn.push(DnType::OrganizationName, "orrbeam");
        dn.push(DnType::OrganizationalUnitName, identity.fingerprint());
        params.distinguished_name = dn;

        // Subject Alternative Names.
        let san_node: rcgen::SanType = SanType::DnsName(
            format!("{}.orrbeam.local", node_name)
                .try_into()
                .map_err(|e: rcgen::Error| TlsError::Rcgen(e))?,
        );
        let san_localhost: rcgen::SanType = SanType::DnsName(
            "localhost"
                .try_into()
                .map_err(|e: rcgen::Error| TlsError::Rcgen(e))?,
        );
        params.subject_alt_names = vec![
            san_node,
            san_localhost,
            SanType::IpAddress(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        ];

        // Validity: 1 day in the past → 20 years in the future.
        use time::{Duration, OffsetDateTime};
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(1);
        params.not_after = now + Duration::days(365 * 20);

        // Generate the certificate (self-signed).
        let cert = params.self_signed(&key_pair)?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        let cert_der = cert.der().to_vec();
        let cert_sha256_hex = Self::sha256_hex(&cert_der);

        // Persist to disk.
        if let Some(parent) = cert_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(cert_path, &cert_pem)?;
        std::fs::write(key_path, &key_pem)?;

        // Restrict key file permissions on Unix (0o600).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600))?;
        }

        info!(
            fingerprint = %cert_sha256_hex,
            cert = %cert_path.display(),
            "TLS identity saved"
        );

        Ok(Self {
            cert_pem,
            key_pem,
            cert_der,
            cert_sha256_hex,
        })
    }

    /// Parse the first certificate DER block out of a PEM string.
    fn der_from_cert_pem(pem: &str) -> Result<Vec<u8>, TlsError> {
        use rustls_pemfile::certs;
        use std::io::BufReader;

        let mut reader = BufReader::new(pem.as_bytes());
        let first = certs(&mut reader)
            .next()
            .ok_or_else(|| TlsError::InvalidKey("no certificate found in PEM".into()))??;
        Ok(first.to_vec())
    }

    /// Compute the lowercase hex SHA-256 of `data`.
    fn sha256_hex(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        hex::encode(hash)
    }

    /// Path to the control-plane certificate PEM file.
    fn cert_path() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("identity").join("control.cert.pem")
    }

    /// Path to the control-plane private key PEM file.
    fn key_path() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("orrbeam").join("identity").join("control.key.pem")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;
    use std::env;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Override the XDG_DATA_HOME env var so that `dirs::data_local_dir()` returns
    /// our temp directory, isolating tests from the real user data dir.
    fn with_temp_data_dir(tmp: &TempDir) -> () {
        // Safety: test-only; single-threaded test setup before any threads are spawned.
        unsafe { env::set_var("XDG_DATA_HOME", tmp.path()) };
    }

    fn make_identity() -> Identity {
        Identity::generate().expect("identity generation failed")
    }

    // ── generate cert + stable fingerprint ───────────────────────────────────

    #[test]
    fn test_generate_cert_and_stable_fingerprint() {
        let tmp = TempDir::new().unwrap();
        with_temp_data_dir(&tmp);

        let id = make_identity();
        let tls1 = TlsIdentity::load_or_create(&id, "test-node")
            .expect("first load_or_create failed");

        assert!(!tls1.cert_sha256_hex.is_empty(), "fingerprint must not be empty");
        assert_eq!(tls1.cert_sha256_hex.len(), 64, "SHA-256 hex must be 64 chars");

        // Recompute fingerprint from the DER bytes and verify consistency.
        let recomputed = TlsIdentity::sha256_hex(&tls1.cert_der);
        assert_eq!(
            tls1.cert_sha256_hex, recomputed,
            "cert_sha256_hex must match sha256(cert_der)"
        );
    }

    // ── idempotency: calling twice gives the same cert ────────────────────────

    #[test]
    fn test_load_or_create_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        with_temp_data_dir(&tmp);

        let id = make_identity();

        let tls1 = TlsIdentity::load_or_create(&id, "idempotent-node")
            .expect("first call failed");
        let tls2 = TlsIdentity::load_or_create(&id, "idempotent-node")
            .expect("second call failed");

        assert_eq!(
            tls1.cert_sha256_hex, tls2.cert_sha256_hex,
            "fingerprint must be stable across two load_or_create calls"
        );
        assert_eq!(
            tls1.cert_pem, tls2.cert_pem,
            "cert PEM must be identical on reload"
        );
    }

    // ── rustls_server_config succeeds ─────────────────────────────────────────

    #[test]
    fn test_rustls_server_config_succeeds() {
        let tmp = TempDir::new().unwrap();
        with_temp_data_dir(&tmp);

        let id = make_identity();
        let tls = TlsIdentity::load_or_create(&id, "rustls-node")
            .expect("load_or_create failed");

        let config = tls.rustls_server_config();
        assert!(
            config.is_ok(),
            "rustls_server_config must succeed; got: {:?}",
            config.err()
        );

        // Confirm the ServerConfig was built successfully (TLS 1.3 is set by the builder).
        let _server_cfg = config.unwrap();
        // Note: rustls::ServerConfig.versions is private; version restriction is
        // enforced by construction via ConfigBuilder::with_protocol_versions.
    }

    // ── Arc<ServerConfig> — config can be shared across threads ──────────────

    #[test]
    fn test_rustls_server_config_is_send_sync() {
        let tmp = TempDir::new().unwrap();
        with_temp_data_dir(&tmp);

        let id = make_identity();
        let tls = TlsIdentity::load_or_create(&id, "thread-node").unwrap();
        let cfg: Arc<ServerConfig> = Arc::new(tls.rustls_server_config().unwrap());

        // Moving Arc<ServerConfig> to another thread must compile and run.
        let cfg2 = Arc::clone(&cfg);
        let handle = std::thread::spawn(move || {
            let _cfg = cfg2;
        });
        handle.join().unwrap();
    }
}
