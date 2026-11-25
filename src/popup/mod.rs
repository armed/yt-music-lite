use std::{fs, path::PathBuf, sync::Mutex};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as port;
// Windows positions windows the same way macOS does.
#[cfg(not(target_os = "linux"))]
mod macos;
#[cfg(not(target_os = "linux"))]
use macos as port;

pub const LABEL: &str = "popup";
const TITLE: &str = "YT Music Popup";
const SIZE: (f64, f64) = (480.0, 200.0);
const GAP: f64 = 6.0;

#[derive(Default)]
struct Placement {
    placed: Option<PhysicalPosition<i32>>,
    saved: Option<PhysicalPosition<i32>>,
    unsaved: bool,
}

struct State(Mutex<Placement>);

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();
    handle.manage(State(Mutex::new(Placement {
        saved: load(handle),
        ..Default::default()
    })));
    port::setup(handle, TITLE, SIZE);

    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title(TITLE)
        .inner_size(SIZE.0, SIZE.1)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .visible(false)
        .build()?;
    Ok(())
}

pub fn show(app: &AppHandle, popup: &WebviewWindow, tray_rect: Option<Rect>) {
    if !port::compositor_places_window() {
        let state = app.state::<State>();
        let saved = state.0.lock().unwrap().saved;
        let position = saved
            .filter(|p| on_screen(app, *p))
            .or_else(|| next_to_tray(app, popup, tray_rect));
        if let Some(position) = position {
            state.0.lock().unwrap().placed = Some(position);
            let _ = popup.set_position(position);
        }
    }
    let _ = popup.show();
    let _ = popup.set_focus();
}

pub fn hide(app: &AppHandle) {
    let Some(popup) = app.get_webview_window(LABEL) else {
        return;
    };
    let _ = popup.hide();
    let state = app.state::<State>();
    let mut placement = state.0.lock().unwrap();
    if std::mem::take(&mut placement.unsaved) {
        save(app, placement.saved);
    }
}

pub fn moved(app: &AppHandle, position: PhysicalPosition<i32>) {
    let state = app.state::<State>();
    let mut placement = state.0.lock().unwrap();
    // set_position reports a move as well, only a drag ends up somewhere else.
    let Some(placed) = placement.placed else {
        return;
    };
    if (placed.x - position.x).abs() <= 1 && (placed.y - position.y).abs() <= 1 {
        return;
    }
    placement.placed = Some(position);
    placement.saved = Some(position);
    placement.unsaved = true;
}

// monitor_from_point takes logical coordinates on macOS, so the saved physical
// position is checked against the monitor rectangles directly.
fn on_screen(app: &AppHandle, p: PhysicalPosition<i32>) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| {
        let (origin, size) = (m.position(), m.size());
        (origin.x..origin.x + size.width as i32).contains(&p.x)
            && (origin.y..origin.y + size.height as i32).contains(&p.y)
    })
}

fn next_to_tray(app: &AppHandle, popup: &WebviewWindow, tray_rect: Option<Rect>) -> Option<PhysicalPosition<i32>> {
    let scale = popup.scale_factor().unwrap_or(1.0);
    let width = SIZE.0 * scale;
    // Before the tray icon has ever been clicked its position is unknown, so
    // fall back to the top-right corner under the menu bar.
    let (pos, size) = match tray_rect {
        Some(rect) => (rect.position.to_physical::<f64>(scale), rect.size.to_physical::<f64>(scale)),
        None => {
            // Wayland has no primary monitor.
            let monitor = app
                .primary_monitor()
                .ok()
                .flatten()
                .or_else(|| app.available_monitors().ok()?.into_iter().next())?;
            let origin = monitor.position();
            let right = origin.x as f64 + monitor.size().width as f64;
            (
                PhysicalPosition::new(right - width / 2.0 - 16.0 * scale, origin.y as f64),
                PhysicalSize::new(0.0, 24.0 * scale),
            )
        }
    };
    let mut x = pos.x + size.width / 2.0 - width / 2.0;
    let y = pos.y + size.height + GAP * scale;

    if let Ok(Some(monitor)) = popup.monitor_from_point(pos.x, pos.y) {
        let left = monitor.position().x as f64;
        let right = left + monitor.size().width as f64;
        x = x.clamp(left + 8.0 * scale, right - width - 8.0 * scale);
    }
    Some(PhysicalPosition::new(x.round() as i32, y.round() as i32))
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("popup.json"))
}

fn load(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let json = fs::read_to_string(path(app)?).ok()?;
    let (x, y): (i32, i32) = serde_json::from_str(&json).ok()?;
    Some(PhysicalPosition::new(x, y))
}

fn save(app: &AppHandle, position: Option<PhysicalPosition<i32>>) {
    let (Some(path), Some(position)) = (path(app), position) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&(position.x, position.y)) {
        let _ = fs::write(path, json);
    }
}
