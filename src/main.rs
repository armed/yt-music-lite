#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod settings;

use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, AppHandle, Emitter, Manager, Rect, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_window_state::StateFlags;

use settings::{Settings, Shortcuts};

const YTM_URL: &str = "https://music.youtube.com";
#[cfg(target_os = "macos")]
const SAFARI_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
(KHTML, like Gecko) Version/18.0 Safari/605.1.15";
#[cfg(target_os = "macos")]
const DATA_STORE_ID: [u8; 16] = *b"ytm-lite-store01";
const POPUP_SIZE: (f64, f64) = (480.0, 200.0);
const POPUP_GAP: f64 = 6.0;
const SETTINGS_SIZE: (f64, f64) = (460.0, 330.0);
const BLUR_TOGGLE_GUARD: Duration = Duration::from_millis(250);
const ACTIONS: &[&str] = &[
    "play",
    "playPause",
    "next",
    "prev",
    "like",
    "dislike",
    "toggleMute",
    "toggleShuffle",
    "toggleRepeat",
    "seek",
];

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("YTM_GIT_SHA"));
const HELP: &str = "\
Usage: ytm-lite [OPTION]

Without options, starts the app (or does nothing if it is already running).

Options:
      --play-pause  Toggle playback
      --next        Skip to the next track
      --prev        Go to the previous track
  -h, --help        Print this help
  -V, --version     Print the version

If the app is running, the command is sent to it. Otherwise the app is started
and the action is applied once a track has loaded.";

const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");
const COLD_START_FLAG: &str = "--cold-start";
const IDENTIFIER: &str = "dev.armed.ytm-lite";

#[cfg(unix)]
fn control_socket_path() -> Option<std::path::PathBuf> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
    };
    Some(base.join(IDENTIFIER).join("control.sock"))
}

#[cfg(unix)]
fn forward_to_running_instance(flag: &str) -> bool {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let Some(path) = control_socket_path() else {
        return false;
    };
    UnixStream::connect(path)
        .and_then(|mut stream| writeln!(stream, "{flag}"))
        .is_ok()
}

#[cfg(unix)]
fn spawn_control_socket() {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixListener;
    let Some(path) = control_socket_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else {
        return;
    };
    let app_cell = std::sync::Arc::clone(&CONTROL_APP);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(stream).read_line(&mut line).is_err() {
                continue;
            }
            let Some(action) = action_from_args(&[line.trim().to_string()]) else {
                continue;
            };
            if let Some(app) = app_cell.lock().unwrap().as_ref() {
                run_action(app, action, None);
            }
        }
    });
}

#[cfg(unix)]
static CONTROL_APP: std::sync::LazyLock<std::sync::Arc<Mutex<Option<AppHandle>>>> =
    std::sync::LazyLock::new(Default::default);

struct AppState {
    player: Mutex<Value>,
    settings: Mutex<Settings>,
    popup_hidden_at: Mutex<Option<Instant>>,
    tray_rect: Mutex<Option<Rect>>,
    pending_action: Mutex<Option<&'static str>>,
    track_seen: AtomicBool,
}

fn action_from_args(args: &[String]) -> Option<&'static str> {
    args.iter().find_map(|arg| match arg.as_str() {
        "--play-pause" => Some("playPause"),
        "--next" => Some("next"),
        "--prev" => Some("prev"),
        _ => None,
    })
}

#[tauri::command]
fn report_state(app: AppHandle, state: Value) {
    let has_track = state
        .get("title")
        .and_then(Value::as_str)
        .is_some_and(|title| !title.is_empty());
    let app_state = app.state::<AppState>();
    *app_state.player.lock().unwrap() = state.clone();
    if has_track {
        app_state.track_seen.store(true, Ordering::Relaxed);
        if let Some(action) = app_state.pending_action.lock().unwrap().take() {
            run_action(&app, if action == "playPause" { "play" } else { action }, None);
        }
    }
    let _ = app.emit_to("popup", "player-state", state);
}

#[tauri::command]
fn get_state(app: AppHandle) -> Value {
    app.state::<AppState>().player.lock().unwrap().clone()
}

#[tauri::command]
fn player_action(app: AppHandle, action: String, value: Option<f64>) {
    run_action(&app, &action, value);
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_start_hidden(app: AppHandle, value: bool) {
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.start_hidden = value;
    settings.save(&app);
}

#[tauri::command]
fn set_shortcut(app: AppHandle, name: String, accelerator: String) -> Result<(), String> {
    let name = Shortcuts::NAMES
        .iter()
        .copied()
        .find(|known| *known == name)
        .ok_or("unknown shortcut")?;
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    let old = settings.shortcuts.get(name).unwrap_or_default().to_string();
    if old == accelerator {
        return Ok(());
    }
    let _ = app.global_shortcut().unregister(old.as_str());
    if let Err(err) = bind_shortcut(&app, name, &accelerator) {
        let _ = bind_shortcut(&app, name, &old);
        return Err(err);
    }
    settings.shortcuts.set(name, accelerator);
    settings.save(&app);
    Ok(())
}

#[tauri::command]
fn get_cli_installed() -> bool {
    cli::is_installed()
}

#[tauri::command]
fn set_cli_installed(value: bool) -> Result<(), String> {
    if value { cli::install() } else { cli::uninstall() }
}

#[tauri::command]
fn reveal_settings_file(app: AppHandle) {
    settings::reveal(&app);
}

#[tauri::command]
fn start_drag(app: AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.start_dragging();
    }
}

#[tauri::command]
fn toggle_maximize(app: AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = if main.is_maximized().unwrap_or(false) {
            main.unmaximize()
        } else {
            main.maximize()
        };
    }
}

#[tauri::command]
fn show_main(app: AppHandle) {
    open_main(&app);
}

fn run_action(app: &AppHandle, action: &str, value: Option<f64>) {
    if !ACTIONS.contains(&action) {
        return;
    }
    let arg = value
        .filter(|v| v.is_finite())
        .map_or_else(String::new, |v| v.to_string());
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.eval(format!("window.__ytm&&window.__ytm.{action}({arg})"));
    }
}

fn open_main(app: &AppHandle) {
    hide_popup(app);
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    // The window stays on the Space it was last shown on, so without this the
    // Dock click would jump back to that Space instead of opening here.
    #[cfg(target_os = "macos")]
    {
        let _ = main.set_visible_on_all_workspaces(true);
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
    }
    let _ = main.show();
    let _ = main.unminimize();
    let _ = main.set_focus();
    #[cfg(target_os = "macos")]
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        let _ = main.set_visible_on_all_workspaces(false);
    });
}

fn hide_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
}

fn hide_popup(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
}

fn toggle_popup(app: &AppHandle, tray_rect: Option<Rect>) {
    let Some(popup) = app.get_webview_window("popup") else {
        return;
    };
    let state = app.state::<AppState>();
    let tray_rect = match tray_rect {
        Some(rect) => {
            *state.tray_rect.lock().unwrap() = Some(rect.clone());
            Some(rect)
        }
        None => state.tray_rect.lock().unwrap().clone(),
    };
    if popup.is_visible().unwrap_or(false) {
        let _ = popup.hide();
        return;
    }
    let recently_blurred = app
        .state::<AppState>()
        .popup_hidden_at
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < BLUR_TOGGLE_GUARD);
    if recently_blurred {
        return;
    }

    let scale = popup.scale_factor().unwrap_or(1.0);
    let width = POPUP_SIZE.0 * scale;
    // Before the tray icon has ever been clicked its position is unknown, so
    // fall back to the top-right corner under the menu bar.
    let (pos, size) = match tray_rect {
        Some(rect) => (
            rect.position.to_physical::<f64>(scale),
            rect.size.to_physical::<f64>(scale),
        ),
        None => {
            let Ok(Some(monitor)) = app.primary_monitor() else {
                return;
            };
            let origin = monitor.position();
            let right = origin.x as f64 + monitor.size().width as f64;
            (
                tauri::PhysicalPosition::new(right - width / 2.0 - 16.0 * scale, origin.y as f64),
                tauri::PhysicalSize::new(0.0, 24.0 * scale),
            )
        }
    };
    let mut x = pos.x + size.width / 2.0 - width / 2.0;
    let y = pos.y + size.height + POPUP_GAP * scale;

    if let Ok(Some(monitor)) = popup.monitor_from_point(pos.x, pos.y) {
        let left = monitor.position().x as f64;
        let right = left + monitor.size().width as f64;
        x = x.clamp(left + 8.0 * scale, right - width - 8.0 * scale);
    }

    let _ = popup.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = popup.show();
    let _ = popup.set_focus();
}

fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    let builder = WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::External(YTM_URL.parse().expect("valid url")),
    )
    .title("YT Music")
    .inner_size(1100.0, 750.0)
    .min_inner_size(600.0, 400.0)
    .background_color(tauri::window::Color(3, 3, 3, 255))
    .visible(false)
    .initialization_script(include_str!("init.js"));
    #[cfg(target_os = "macos")]
    let builder = builder
        .data_store_identifier(DATA_STORE_ID)
        .user_agent(SAFARI_UA)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true);
    builder.build()?;
    Ok(())
}

fn build_popup_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("index.html".into()))
        .inner_size(POPUP_SIZE.0, POPUP_SIZE.1)
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

fn build_settings_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("YT Music Lite Settings")
        .inner_size(SETTINGS_SIZE.0, SETTINGS_SIZE.1)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .center()
        .visible(false)
        .build()?;
    Ok(())
}

fn open_settings(app: &AppHandle) {
    hide_popup(app);
    let Some(window) = app.get_webview_window("settings") else {
        return;
    };
    let _ = window.show();
    let _ = window.set_focus();
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .item(&MenuItem::with_id(app, "open", "Open YT Music", true, None::<&str>)?)
        .item(&MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?)
        .separator()
        .item(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)
        .build()?;

    TrayIconBuilder::with_id("tray")
        .icon(Image::from_bytes(TRAY_ICON)?)
        .tooltip("YT Music")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => open_main(app),
            "quit" => app.exit(0),
            "settings" => open_settings(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_popup(tray.app_handle(), Some(rect));
            }
        })
        .build(app)?;
    Ok(())
}

fn spawn_loading_indicator(app: AppHandle) {
    const FRAMES: [&[u8]; 6] = [
        include_bytes!("../icons/tray-loading-0.png"),
        include_bytes!("../icons/tray-loading-1.png"),
        include_bytes!("../icons/tray-loading-2.png"),
        include_bytes!("../icons/tray-loading-3.png"),
        include_bytes!("../icons/tray-loading-4.png"),
        include_bytes!("../icons/tray-loading-5.png"),
    ];
    const GIVE_UP_AFTER: Duration = Duration::from_secs(25);
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut frame = 0;
        loop {
            let state = app.state::<AppState>();
            let Some(tray) = app.tray_by_id("tray") else {
                return;
            };
            let seen = state.track_seen.load(Ordering::Relaxed);
            if seen || started.elapsed() > GIVE_UP_AFTER {
                if !seen {
                    *state.pending_action.lock().unwrap() = None;
                }
                if let Ok(icon) = Image::from_bytes(TRAY_ICON) {
                    let _ = tray.set_icon(Some(icon));
                }
                let _ = tray.set_tooltip(Some("YT Music"));
                return;
            }
            if let Ok(icon) = Image::from_bytes(FRAMES[frame % FRAMES.len()]) {
                let _ = tray.set_icon(Some(icon));
            }
            let _ = tray.set_tooltip(Some("YT Music — loading…"));
            frame += 1;
            std::thread::sleep(Duration::from_millis(150));
        }
    });
}

fn bind_shortcut(app: &AppHandle, name: &'static str, accelerator: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(accelerator, move |app, _, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            match name {
                "play_pause" => run_action(app, "playPause", None),
                "next" => run_action(app, "next", None),
                "prev" => run_action(app, "prev", None),
                "toggle_popup" => toggle_popup(app, None),
                _ => {}
            }
        })
        .map_err(|err| err.to_string())
}

fn register_shortcuts(app: &AppHandle, settings: &Settings) {
    for &name in Shortcuts::NAMES {
        let accelerator = settings.shortcuts.get(name).unwrap_or_default();
        if let Err(err) = bind_shortcut(app, name, accelerator) {
            eprintln!("cannot register shortcut {accelerator:?}: {err}");
        }
    }
}

#[cfg(target_os = "macos")]
fn launch_via_launch_services(flag: &str) -> bool {
    let Ok(exe) = std::env::current_exe().and_then(std::fs::canonicalize) else {
        return false;
    };
    let Some(bundle) = exe.ancestors().nth(3) else {
        return false;
    };
    if bundle.extension().is_none_or(|ext| ext != "app") {
        return false;
    }
    std::process::Command::new("/usr/bin/open")
        .args(["-g", "-a"])
        .arg(bundle)
        .args(["--args", flag, COLD_START_FLAG])
        .status()
        .is_ok_and(|status| status.success())
}

fn main() {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("ytm-lite {VERSION}\n\n{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("ytm-lite {VERSION}");
                return;
            }
            _ => {}
        }
    }

    #[cfg(unix)]
    if std::env::args().all(|a| a != COLD_START_FLAG)
        && let Some(flag) = std::env::args().skip(1).find(|a| action_from_args(std::slice::from_ref(a)).is_some()) {
        if forward_to_running_instance(&flag) {
            return;
        }
        #[cfg(target_os = "macos")]
        if launch_via_launch_services(&flag) {
            return;
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(action) = action_from_args(&args) {
                run_action(app, action, None);
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .with_denylist(&["popup", "settings"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            report_state,
            get_state,
            player_action,
            show_main,
            get_settings,
            set_start_hidden,
            set_shortcut,
            reveal_settings_file,
            get_cli_installed,
            set_cli_installed,
            start_drag,
            toggle_maximize
        ])
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                hide_main(window.app_handle());
            }
            ("settings", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("popup", WindowEvent::Focused(false)) => {
                let app = window.app_handle();
                *app.state::<AppState>().popup_hidden_at.lock().unwrap() = Some(Instant::now());
                hide_popup(app);
            }
            _ => {}
        })
        .setup(|app| {
            let settings = Settings::load(app.handle());
            let start_hidden = settings.start_hidden;
            register_shortcuts(app.handle(), &settings);
            app.manage(AppState {
                player: Mutex::new(Value::Null),
                settings: Mutex::new(settings),
                popup_hidden_at: Mutex::new(None),
                tray_rect: Mutex::new(None),
                pending_action: Mutex::new(action_from_args(
                    &std::env::args().collect::<Vec<_>>(),
                )),
                track_seen: AtomicBool::new(false),
            });

            #[cfg(unix)]
            {
                *CONTROL_APP.lock().unwrap() = Some(app.handle().clone());
                spawn_control_socket();
            }

            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            build_main_window(app)?;
            build_popup_window(app)?;
            build_settings_window(app)?;
            build_tray(app)?;
            spawn_loading_indicator(app.handle().clone());

            if !start_hidden {
                open_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build app")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                open_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
