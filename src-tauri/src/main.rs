#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK workarounds for Linux Wayland.
    // DMABUF renderer can cause blank/frozen webview on some GPU/driver combos.
    // Compositing mode issues can cause input events to not register.
    #[cfg(target_os = "linux")]
    {
        if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            }
        }
    }

    orrbeam_app::run();
}
