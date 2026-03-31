#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK DMABUF workaround for Linux Wayland
    #[cfg(target_os = "linux")]
    {
        if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

    orrbeam_app::run();
}
