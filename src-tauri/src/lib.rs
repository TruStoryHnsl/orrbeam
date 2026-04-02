mod commands;

use orrbeam_core::{Config, Identity, NodeRegistry};
use orrbeam_net::DiscoveryManager;
use orrbeam_platform::get_platform;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state accessible from all Tauri commands.
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub identity: Identity,
    pub registry: Arc<RwLock<NodeRegistry>>,
    pub platform: Box<dyn orrbeam_platform::Platform>,
}

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

    let identity = Identity::load_or_create().expect("failed to initialize identity");
    tracing::info!("node identity: {}", identity.fingerprint());

    let registry = Arc::new(RwLock::new(NodeRegistry::new()));
    let platform = get_platform();

    let state = AppState {
        config: Arc::new(RwLock::new(config.clone())),
        identity,
        registry: registry.clone(),
        platform,
    };

    // Start discovery in background
    let discovery_config = config.clone();
    let discovery_registry = registry.clone();
    tauri::async_runtime::spawn(async move {
        let manager = DiscoveryManager::new(discovery_config, discovery_registry);
        if let Err(e) = manager.start().await {
            tracing::error!("discovery failed to start: {e}");
        }
    });

    tauri::Builder::default()
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
            commands::discovery::get_nodes,
            commands::discovery::get_node_count,
            commands::settings::get_config,
            commands::settings::save_config,
            commands::settings::get_identity,
        ])
        .run(tauri::generate_context!())
        .expect("error while running orrbeam");
}
