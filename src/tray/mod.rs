use tauri::{AppHandle, Rect};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as port;
// Windows reports tray clicks through Tauri the same way macOS does.
#[cfg(not(target_os = "linux"))]
mod macos;
#[cfg(not(target_os = "linux"))]
use macos as port;

pub const ICON: &[u8] = include_bytes!("../../icons/tray.png");
pub const TOOLTIP: &str = "YT Music";

enum Entry {
    Item(&'static str, &'static str),
    Separator,
}

fn menu() -> Vec<Entry> {
    let mut entries = vec![Entry::Item("open", "Open YT Music")];
    if port::POPUP_MENU_ITEM {
        entries.push(Entry::Item("popup", "Mini player"));
    }
    entries.extend([
        Entry::Item("settings", "Settings…"),
        Entry::Separator,
        Entry::Item("quit", "Quit"),
    ]);
    entries
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => crate::open_main(app),
        "popup" => crate::toggle_popup(app, None),
        "settings" => crate::open_settings(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn on_click(app: &AppHandle, rect: Option<Rect>) {
    crate::toggle_popup(app, rect);
}

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    port::build(app)
}

pub fn set_icon(app: &AppHandle, png: &[u8], tooltip: &str) {
    port::set_icon(app, png, tooltip);
}
