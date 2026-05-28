#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKit's DMA-BUF renderer prevents all keyboard input on Wayland and some
    // X11 compositors. Must be set before the Tauri/GTK runtime initialises.
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    hermes_webui_desktop_lib::run()
}
