use std::{fs, path::PathBuf, process::Command};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Shortcuts {
    pub play_pause: String,
    pub next: String,
    pub prev: String,
    pub toggle_popup: String,
}

impl Shortcuts {
    pub const NAMES: &[&str] = &["play_pause", "next", "prev", "toggle_popup"];

    pub fn get(&self, name: &str) -> Option<&str> {
        match name {
            "play_pause" => Some(&self.play_pause),
            "next" => Some(&self.next),
            "prev" => Some(&self.prev),
            "toggle_popup" => Some(&self.toggle_popup),
            _ => None,
        }
    }

    pub fn set(&mut self, name: &str, value: String) {
        match name {
            "play_pause" => self.play_pause = value,
            "next" => self.next = value,
            "prev" => self.prev = value,
            "toggle_popup" => self.toggle_popup = value,
            _ => {}
        }
    }
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            play_pause: "CmdOrCtrl+Alt+P".into(),
            next: "CmdOrCtrl+Alt+Right".into(),
            prev: "CmdOrCtrl+Alt+Left".into(),
            toggle_popup: "CmdOrCtrl+Alt+M".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub start_hidden: bool,
    pub shortcuts: Shortcuts,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_hidden: true,
            shortcuts: Shortcuts::default(),
        }
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("settings.json"))
}

impl Settings {
    pub fn load(app: &AppHandle) -> Self {
        let settings: Self = path(app)
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        settings.save(app);
        settings
    }

    pub fn save(&self, app: &AppHandle) {
        let Some(path) = path(app) else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

pub fn reveal(app: &AppHandle) {
    let Some(path) = path(app) else { return };
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = Command::new(opener).arg(path).spawn();
}
