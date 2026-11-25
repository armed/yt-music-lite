mod kwin;

use tauri::AppHandle;

pub fn setup(app: &AppHandle, title: &str, size: (f64, f64)) {
    kwin::ensure_popup_rule(app, title, size);
}

pub fn compositor_places_window() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}
