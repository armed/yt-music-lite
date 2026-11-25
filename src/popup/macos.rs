use tauri::AppHandle;

pub fn setup(_app: &AppHandle, _title: &str, _size: (f64, f64)) {}

pub fn compositor_places_window() -> bool {
    false
}
