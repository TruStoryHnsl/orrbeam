//! Integration tests for orrbeam-net control-plane server.
//!
//! These tests spin up the control server on a random port, make HTTP requests
//! over HTTPS (accepting the self-signed cert for testing), and verify the
//! response shapes.
//!
//! Real-hardware tests (mDNS, actual peer-to-peer) are marked `#[ignore]`.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use orrbeam_core::config::Config;
use orrbeam_core::identity::Identity;
use orrbeam_core::tls::TlsIdentity;
use orrbeam_net::server::{ControlState, NoopEmitter, NonceCache};

// Serialize tests that mutate XDG_DATA_HOME to avoid env var races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("local_addr").port()
}

struct StubPlatform;

impl orrbeam_platform::Platform for StubPlatform {
    fn info(&self) -> orrbeam_platform::PlatformInfo {
        orrbeam_platform::PlatformInfo {
            os: "test".into(),
            os_version: None,
            display_server: None,
            hostname: "test-node".into(),
        }
    }
    fn sunshine_status(&self, _: &Config) -> Result<orrbeam_platform::ServiceInfo, orrbeam_platform::PlatformError> {
        Ok(orrbeam_platform::ServiceInfo {
            name: "sunshine".into(),
            status: orrbeam_platform::ServiceStatus::NotInstalled,
            version: None,
            path: None,
        })
    }
    fn moonlight_status(&self, _: &Config) -> Result<orrbeam_platform::ServiceInfo, orrbeam_platform::PlatformError> {
        Ok(orrbeam_platform::ServiceInfo {
            name: "moonlight".into(),
            status: orrbeam_platform::ServiceStatus::NotInstalled,
            version: None,
            path: None,
        })
    }
    fn start_sunshine(&self, _: &Config) -> Result<(), orrbeam_platform::PlatformError> { Ok(()) }
    fn stop_sunshine(&self) -> Result<(), orrbeam_platform::PlatformError> { Ok(()) }
    fn start_moonlight(&self, _: &Config, _: &str, _: &str, _: bool, _: Option<&str>) -> Result<(), orrbeam_platform::PlatformError> { Ok(()) }
    fn stop_moonlight(&self) -> Result<(), orrbeam_platform::PlatformError> { Ok(()) }
    fn monitors(&self) -> Result<Vec<orrbeam_platform::MonitorInfo>, orrbeam_platform::PlatformError> { Ok(vec![]) }
    fn gpu_info(&self) -> Result<orrbeam_platform::GpuInfo, orrbeam_platform::PlatformError> {
        Ok(orrbeam_platform::GpuInfo { name: "stub".into(), encoder: "stub".into(), driver: None })
    }
    fn pair_moonlight(&self, _: &Config, _: &str, _: &str) -> Result<(), orrbeam_platform::PlatformError> { Ok(()) }
}

/// Spin up a test control server. Returns (addr, shutdown_token).
/// The TempDir must live for the duration of the test — keep it bound.
async fn spawn_test_server() -> (SocketAddr, CancellationToken, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().expect("tempdir");

    let identity;
    let tls;
    {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp.path()) };
        identity = Arc::new(Identity::generate().expect("identity"));
        tls = Arc::new(TlsIdentity::load_or_create(&identity, "test-node").expect("tls"));
    }

    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let shutdown = CancellationToken::new();

    let state = Arc::new(ControlState {
        identity,
        tls,
        config: Arc::new(RwLock::new(Config::default())),
        peers: Arc::new(RwLock::new(orrbeam_core::peers::TrustedPeerStore::default())),
        nonces: NonceCache::new(),
        pending_mutual_trust: Arc::new(RwLock::new(HashMap::new())),
        platform: Arc::new(StubPlatform),
        event_emitter: Arc::new(NoopEmitter),
        shutdown: shutdown.clone(),
    });

    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        if let Err(e) = orrbeam_net::server::serve(state, addr).await {
            if !shutdown_clone.is_cancelled() {
                eprintln!("test server error: {e}");
            }
        }
    });

    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (addr, shutdown, tmp)
}

fn tofu_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hello_endpoint_returns_200() {
    let (addr, shutdown, _tmp) = spawn_test_server().await;
    let resp = tofu_client()
        .get(format!("https://{addr}/v1/hello"))
        .send()
        .await
        .expect("GET /v1/hello");
    assert_eq!(resp.status(), 200);
    shutdown.cancel();
}

#[tokio::test]
async fn hello_endpoint_has_required_fields() {
    let (addr, shutdown, _tmp) = spawn_test_server().await;
    let body: serde_json::Value = tofu_client()
        .get(format!("https://{addr}/v1/hello"))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .expect("parse JSON");

    assert!(body.get("node_name").is_some(), "missing node_name");
    assert!(body.get("ed25519_fingerprint").is_some(), "missing ed25519_fingerprint");
    assert!(body.get("cert_sha256").is_some(), "missing cert_sha256");
    assert!(body.get("control_port").is_some(), "missing control_port");

    shutdown.cancel();
}

#[tokio::test]
async fn authenticated_route_without_headers_returns_error() {
    let (addr, shutdown, _tmp) = spawn_test_server().await;
    let resp = tofu_client()
        .get(format!("https://{addr}/v1/status"))
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 401,
        "expected 400/401 for unauthenticated /v1/status, got {status}"
    );

    shutdown.cancel();
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let (addr, shutdown, _tmp) = spawn_test_server().await;
    let resp = tofu_client()
        .get(format!("https://{addr}/v1/nonexistent-route"))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 404);
    shutdown.cancel();
}

// ---------------------------------------------------------------------------
// Real-hardware tests (ignored by default)
// ---------------------------------------------------------------------------

/// Requires a real mDNS-capable network.
#[tokio::test]
#[ignore]
async fn mdns_browse_does_not_panic() {
    let registry = Arc::new(RwLock::new(orrbeam_core::node::NodeRegistry::new()));
    // Spawn browse; it runs indefinitely — abort after a short delay.
    let handle = tokio::spawn(orrbeam_net::mdns::browse(registry));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    handle.abort();
}
