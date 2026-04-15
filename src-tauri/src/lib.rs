mod commands;
mod tray;

use orrbeam_core::{Config, Identity, NodeRegistry};
use orrbeam_net::DiscoveryManager;
use orrbeam_platform::get_platform;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Shared application state accessible from all Tauri commands.
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub identity: Arc<Identity>,
    pub registry: Arc<RwLock<NodeRegistry>>,
    pub platform: Arc<dyn orrbeam_platform::Platform + Send + Sync>,
    pub tls: Arc<orrbeam_core::tls::TlsIdentity>,
    pub peers: Arc<RwLock<orrbeam_core::peers::TrustedPeerStore>>,
    /// In-flight mutual-trust TOFU requests keyed by UUID.
    ///
    /// This `Arc` is shared with [`orrbeam_net::server::ControlState`] so that
    /// the server's inbound-request handlers and the Tauri command layer both
    /// read/write the same map.
    pub pending_mutual_trust: Arc<RwLock<std::collections::HashMap<uuid::Uuid, orrbeam_net::server::PendingMutualTrust>>>,
    pub control_shutdown: CancellationToken,
}

// ---------------------------------------------------------------------------
// TauriEventEmitter — forwards control-plane events to the frontend
// ---------------------------------------------------------------------------

struct TauriEventEmitter {
    handle: tauri::AppHandle,
}

#[async_trait::async_trait]
impl orrbeam_net::server::EventEmitter for TauriEventEmitter {
    async fn emit(&self, topic: &str, payload: serde_json::Value) {
        if let Err(e) = self.handle.emit(topic, payload) {
            tracing::warn!(topic, "failed to emit tauri event: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// run()
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "orrbeam=info".into()),
        )
        .init();

    let config = Config::load().unwrap_or_else(|e| {
        tracing::warn!("failed to load config, using defaults: {e}");
        Config::default()
    });

    let identity = Arc::new(
        Identity::load_or_create().expect("failed to initialize identity"),
    );
    tracing::info!("node identity: {}", identity.fingerprint());

    // Load TLS identity (cert derived from Ed25519 key).
    let tls = Arc::new(
        orrbeam_core::tls::TlsIdentity::load_or_create(&identity, &config.node_name)
            .expect("failed to initialize TLS identity"),
    );
    tracing::info!(cert_sha256 = %tls.cert_sha256_hex, "TLS identity ready");

    // Load trusted peer store.
    let peers = Arc::new(RwLock::new(
        orrbeam_core::peers::TrustedPeerStore::load().unwrap_or_else(|e| {
            tracing::warn!("failed to load trusted peers, starting fresh: {e}");
            orrbeam_core::peers::TrustedPeerStore::default()
        }),
    ));

    // Cancellation token used to stop the control server on app exit.
    let control_shutdown = CancellationToken::new();

    // Shared map for in-flight mutual-trust TOFU requests.  The same Arc is
    // given to both AppState (for Tauri commands) and ControlState (for the
    // HTTP server inbound handler) so they always operate on the same data.
    let pending_mutual_trust: Arc<RwLock<HashMap<uuid::Uuid, orrbeam_net::server::PendingMutualTrust>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Load the persistent registry from disk; fall back to empty on first run.
    let persisted_registry = {
        let path = NodeRegistry::default_path();
        NodeRegistry::load(&path).unwrap_or_else(|e| {
            tracing::warn!("failed to load node registry, starting fresh: {e}");
            NodeRegistry::new()
        })
    };
    let registry = Arc::new(RwLock::new(persisted_registry));
    let platform = get_platform();
    let config_arc = Arc::new(RwLock::new(config.clone()));

    let state = AppState {
        config: config_arc.clone(),
        identity: identity.clone(),
        registry: registry.clone(),
        platform: platform.clone(),
        tls: tls.clone(),
        peers: peers.clone(),
        pending_mutual_trust: pending_mutual_trust.clone(),
        control_shutdown: control_shutdown.clone(),
    };

    // Start discovery in background.
    {
        let discovery_config = config.clone();
        let discovery_registry = registry.clone();
        let discovery_identity = identity.clone();
        let discovery_tls = tls.clone();
        let discovery_platform = platform.clone();
        let api_port = config.api_port;

        tauri::async_runtime::spawn(async move {
            let gpu_encoder = discovery_platform.gpu_info().ok().map(|g| g.encoder);
            let os = discovery_platform.info().os;
            let sunshine_available = discovery_platform
                .sunshine_status(&discovery_config)
                .map(|s| {
                    matches!(
                        s.status,
                        orrbeam_platform::ServiceStatus::Running
                            | orrbeam_platform::ServiceStatus::Installed
                    )
                })
                .unwrap_or(false);
            let moonlight_available = discovery_platform
                .moonlight_status(&discovery_config)
                .map(|s| {
                    matches!(
                        s.status,
                        orrbeam_platform::ServiceStatus::Running
                            | orrbeam_platform::ServiceStatus::Installed
                    )
                })
                .unwrap_or(false);

            let reg_info = orrbeam_net::RegistrationInfo {
                fingerprint: discovery_identity.fingerprint(),
                cert_sha256: discovery_tls.cert_sha256_hex.clone(),
                sunshine_available,
                moonlight_available,
                os,
                encoder: gpu_encoder,
                port: api_port,
            };

            let mut manager = DiscoveryManager::new(discovery_config, discovery_registry);
            if let Err(e) = manager.start(Some(reg_info)).await {
                tracing::error!("discovery failed to start: {e}");
            }
        });
    }

    // Build the Tauri application.  The control server is spawned inside
    // .setup() so that we have an AppHandle for TauriEventEmitter.
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::platform::get_platform_info,
            commands::platform::get_gpu_info,
            commands::platform::get_monitors,
            commands::sunshine::get_sunshine_status,
            commands::sunshine::start_sunshine,
            commands::sunshine::stop_sunshine,
            commands::sunshine::get_sunshine_settings,
            commands::sunshine::set_sunshine_settings,
            commands::sunshine::set_sunshine_monitor,
            commands::moonlight::get_moonlight_status,
            commands::moonlight::start_moonlight,
            commands::moonlight::stop_moonlight,
            commands::pairing::pair_initiate,
            commands::pairing::pair_accept,
            commands::remote::list_trusted_peers,
            commands::remote::fetch_peer_hello,
            commands::remote::confirm_trusted_peer,
            commands::remote::remove_trusted_peer,
            commands::remote::update_peer_permissions,
            commands::remote::request_mutual_trust,
            commands::remote::approve_mutual_trust_request,
            commands::remote::reject_mutual_trust_request,
            commands::remote::list_inbound_mutual_trust_requests,
            commands::remote::connect_to_peer,
            commands::remote::remote_peer_status,
            commands::discovery::get_nodes,
            commands::discovery::get_node_count,
            commands::discovery::add_node,
            commands::discovery::remove_node,
            commands::discovery::list_nodes,
            commands::settings::get_config,
            commands::settings::save_config,
            commands::settings::get_identity,
            commands::settings::get_tls_fingerprint,
        ])
        .setup(|app| {
            let app_state = app.state::<AppState>();

            // Build a TauriEventEmitter now that we have an AppHandle.
            let emitter = Arc::new(TauriEventEmitter {
                handle: app.handle().clone(),
            });

            // Build ControlState with all required fields.
            // `pending_mutual_trust` is the *same* Arc held in AppState so that
            // the HTTP server and Tauri commands share the live request map.
            let control_state = Arc::new(orrbeam_net::server::ControlState {
                identity: app_state.identity.clone(),
                tls: app_state.tls.clone(),
                config: app_state.config.clone(),
                peers: app_state.peers.clone(),
                nonces: orrbeam_net::server::NonceCache::new(),
                pending_mutual_trust: app_state.pending_mutual_trust.clone(),
                platform: app_state.platform.clone(),
                event_emitter: emitter,
                shutdown: app_state.control_shutdown.clone(),
            });

            // Determine bind address from config.
            let bind_addr: std::net::SocketAddr = {
                let cfg = app_state.config.try_read().expect("config locked in setup");
                format!("{}:{}", cfg.api_bind, cfg.api_port)
                    .parse()
                    .expect("invalid api_bind:api_port in config")
            };

            // Spawn the nonce GC task.
            control_state
                .nonces
                .clone()
                .spawn_gc(app_state.control_shutdown.clone());

            // Spawn the control server in the Tauri async runtime.
            let server_state = control_state;
            tauri::async_runtime::spawn(async move {
                tracing::info!(%bind_addr, "control server starting");
                if let Err(e) = orrbeam_net::server::serve(server_state, bind_addr).await {
                    tracing::error!("control server exited: {e}");
                }
            });

            tray::create_tray(app)?;
            tray::spawn_tray_updater(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                tray::refresh_tray(window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("error building orrbeam");

    // Run the event loop.  Cancel the shutdown token on exit so the control
    // server drains cleanly.
    app.run(move |_handle, event| {
        if let tauri::RunEvent::Exit = event {
            control_shutdown.cancel();
        }
    });
}
