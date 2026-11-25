// Wayland ignores client-side window positions and skip-taskbar hints, so on
// KDE the popup is hidden from the taskbar by a KWin window rule, and KWin
// remembers where the popup was dragged to.

use std::process::Command;

use tauri::AppHandle;

const RULES_FILE: &str = "kwinrulesrc";
const RULE_ID: &str = "ytm-lite-popup";
const FORCE: &str = "2";
const REMEMBER: &str = "4";
const EXACT_MATCH: &str = "1";
const MARGIN: f64 = 8.0;
// Room for the default floating Plasma panel at the bottom of the screen.
const PANEL_ALLOWANCE: f64 = 64.0;

pub fn ensure_popup_rule(app: &AppHandle, title: &str, popup_size: (f64, f64)) {
    if !is_kde_wayland() || rule_exists() {
        return;
    }
    let Some(position) = bottom_right(app, popup_size) else {
        return;
    };
    let keys = [
        ("Description", title),
        ("title", title),
        ("titlematch", EXACT_MATCH),
        ("position", &position),
        ("positionrule", REMEMBER),
        ("above", "true"),
        ("aboverule", FORCE),
        ("skiptaskbar", "true"),
        ("skiptaskbarrule", FORCE),
        ("skipswitcher", "true"),
        ("skipswitcherrule", FORCE),
    ];
    for (key, value) in keys {
        if let Err(err) = write(RULE_ID, key, value) {
            eprintln!("cannot write KWin rule: {err}");
            return;
        }
    }

    // Older KWin lists rules in "rules" and "count", Plasma 6 rewrites them to "Order".
    let rules = read("General", "Order")
        .filter(|ids| !ids.is_empty())
        .or_else(|| read("General", "rules"))
        .unwrap_or_default();
    let mut ids: Vec<&str> = rules.split(',').filter(|id| !id.is_empty() && *id != RULE_ID).collect();
    ids.push(RULE_ID);
    let ids = ids.join(",");
    let _ = write("General", "Order", &ids);
    let _ = write("General", "rules", &ids);
    let _ = write("General", "count", &ids.split(',').count().to_string());

    let _ = Command::new("gdbus")
        .args(["call", "--session", "--dest", "org.kde.KWin", "--object-path", "/KWin"])
        .args(["--method", "org.kde.KWin.reconfigure"])
        .output();
}

fn is_kde_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| desktop.split(':').any(|d| d == "KDE"))
}

fn rule_exists() -> bool {
    read(RULE_ID, "title").is_some_and(|title| !title.is_empty())
}

fn bottom_right(app: &AppHandle, (width, height): (f64, f64)) -> Option<String> {
    let monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| app.available_monitors().ok()?.into_iter().next())?;
    let scale = monitor.scale_factor();
    let origin = monitor.position().to_logical::<f64>(scale);
    let size = monitor.size().to_logical::<f64>(scale);
    let x = origin.x + size.width - width - MARGIN;
    let y = origin.y + size.height - height - PANEL_ALLOWANCE;
    Some(format!("{},{}", x.round(), y.round()))
}

fn read(group: &str, key: &str) -> Option<String> {
    let output = Command::new("kreadconfig6")
        .args(["--file", RULES_FILE, "--group", group, "--key", key])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn write(group: &str, key: &str, value: &str) -> Result<(), String> {
    let status = Command::new("kwriteconfig6")
        .args(["--file", RULES_FILE, "--group", group, "--key", key, value])
        .status()
        .map_err(|err| format!("kwriteconfig6: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kwriteconfig6 exited with {status}"))
    }
}
