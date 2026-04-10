use crate::AppState;
use orrbeam_platform::ServiceStatus;
use tauri::{
    image::Image,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Wry,
};

/// Create the system tray icon with an initial menu.
pub fn create_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let toggle = MenuItem::with_id(app, "toggle_visibility", "Hide Orrbeam", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sunshine_status =
        MenuItem::with_id(app, "sunshine_status", "\u{2600} Sunshine: --", false, None::<&str>)?;
    let moonlight_status = MenuItem::with_id(
        app,
        "moonlight_status",
        "\u{1f319} Moonlight: --",
        false,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let connect_header = MenuItem::with_id(
        app,
        "connect_header",
        "Connect to...",
        false,
        None::<&str>,
    )?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &sep1,
            &sunshine_status,
            &moonlight_status,
            &sep2,
            &connect_header,
            &sep3,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/32x32.png"))?)
        .tooltip("Orrbeam")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app: &AppHandle<Wry>, event: MenuEvent| {
            handle_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray: &TrayIcon<Wry>, event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Handle tray menu item clicks.
fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "toggle_visibility" => {
            if let Some(window) = app.get_webview_window("main") {
                let visible = window.is_visible().unwrap_or(true);
                if visible {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
                refresh_tray(app);
            }
        }
        "quit" => {
            app.exit(0);
        }
        other if other.starts_with("connect_") => {
            let address = other.strip_prefix("connect_").unwrap_or("");
            if address.is_empty() {
                return;
            }
            let state = app.state::<AppState>();
            let config_lock = state.config.clone();
            let platform_status = {
                if let Ok(config) = config_lock.try_read() {
                    Some(config.clone())
                } else {
                    None
                }
            };
            if let Some(config) = platform_status
                && let Err(e) =
                    state
                        .platform
                        .start_moonlight(&config, address, "Desktop", false, None)
            {
                tracing::error!("failed to connect to {address}: {e}");
            }
        }
        _ => {}
    }
}

/// Refresh the tray menu with current state (service status, online nodes).
pub fn refresh_tray(app_handle: &AppHandle) {
    if let Err(e) = refresh_tray_inner(app_handle) {
        tracing::error!("failed to refresh tray menu: {e}");
    }
}

fn refresh_tray_inner(app_handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = app_handle.state::<AppState>();

    // Read config (non-blocking)
    let config = match state.config.try_read() {
        Ok(c) => c.clone(),
        Err(_) => {
            tracing::debug!("config locked, skipping tray refresh");
            return Ok(());
        }
    };

    // Read node registry (non-blocking)
    let online_nodes = match state.registry.try_read() {
        Ok(reg) => reg
            .online()
            .into_iter()
            .map(|n| (n.name.clone(), n.address.to_string(), n.os.clone()))
            .collect::<Vec<_>>(),
        Err(_) => {
            tracing::debug!("registry locked, skipping tray refresh");
            return Ok(());
        }
    };

    // Get service statuses
    let sunshine_label = match state.platform.sunshine_status(&config) {
        Ok(info) => format!("\u{2600} Sunshine: {}", status_label(&info.status)),
        Err(_) => "\u{2600} Sunshine: Unknown".to_string(),
    };
    let moonlight_label = match state.platform.moonlight_status(&config) {
        Ok(info) => format!("\u{1f319} Moonlight: {}", status_label(&info.status)),
        Err(_) => "\u{1f319} Moonlight: Unknown".to_string(),
    };

    // Determine toggle label
    let toggle_label = if let Some(window) = app_handle.get_webview_window("main") {
        if window.is_visible().unwrap_or(true) {
            "Hide Orrbeam"
        } else {
            "Show Orrbeam"
        }
    } else {
        "Show Orrbeam"
    };

    // Build menu items
    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = Vec::new();

    let toggle =
        MenuItem::with_id(app_handle, "toggle_visibility", toggle_label, true, None::<&str>)?;
    items.push(Box::new(toggle));

    items.push(Box::new(PredefinedMenuItem::separator(app_handle)?));

    let sun =
        MenuItem::with_id(app_handle, "sunshine_status", &sunshine_label, false, None::<&str>)?;
    items.push(Box::new(sun));

    let moon = MenuItem::with_id(
        app_handle,
        "moonlight_status",
        &moonlight_label,
        false,
        None::<&str>,
    )?;
    items.push(Box::new(moon));

    items.push(Box::new(PredefinedMenuItem::separator(app_handle)?));

    if online_nodes.is_empty() {
        let empty = MenuItem::with_id(
            app_handle,
            "no_nodes",
            "No nodes online",
            false,
            None::<&str>,
        )?;
        items.push(Box::new(empty));
    } else {
        let header = MenuItem::with_id(
            app_handle,
            "connect_header",
            "Connect to...",
            false,
            None::<&str>,
        )?;
        items.push(Box::new(header));

        for (name, address, os) in &online_nodes {
            let os_str = os.as_deref().unwrap_or("unknown");
            let label = format!("{name} ({os_str})");
            let id = format!("connect_{address}");
            let item = MenuItem::with_id(app_handle, &id, &label, true, None::<&str>)?;
            items.push(Box::new(item));
        }
    }

    items.push(Box::new(PredefinedMenuItem::separator(app_handle)?));

    let quit = MenuItem::with_id(app_handle, "quit", "Quit", true, None::<&str>)?;
    items.push(Box::new(quit));

    // Build the menu from references
    let item_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        items.iter().map(|i| i.as_ref()).collect();
    let menu = Menu::with_items(app_handle, &item_refs)?;

    // Apply to tray
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }

    tracing::debug!(
        "tray refreshed: sunshine={}, moonlight={}, nodes={}",
        sunshine_label,
        moonlight_label,
        online_nodes.len()
    );

    Ok(())
}

/// Map ServiceStatus to a human-readable label.
fn status_label(status: &ServiceStatus) -> &'static str {
    match status {
        ServiceStatus::Running => "Running",
        ServiceStatus::Installed => "Stopped",
        ServiceStatus::NotInstalled => "Not Installed",
        ServiceStatus::Unknown => "Unknown",
    }
}

/// Spawn a background task that refreshes the tray every 5 seconds.
pub fn spawn_tray_updater(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            refresh_tray(&app_handle);
        }
    });
}
