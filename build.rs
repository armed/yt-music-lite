fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "report_state",
        "get_state",
        "player_action",
        "show_main",
        "get_settings",
        "set_start_hidden",
        "set_shortcut",
        "reveal_settings_file",
        "start_drag",
        "toggle_maximize",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
