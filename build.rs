fn main() {
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=YTM_GIT_SHA={sha}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let manifest = tauri_build::AppManifest::new().commands(&[
        "report_state",
        "get_state",
        "player_action",
        "show_main",
        "get_settings",
        "set_start_hidden",
        "set_shortcut",
        "reveal_settings_file",
        "get_cli_installed",
        "set_cli_installed",
        "start_drag",
        "toggle_maximize",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
