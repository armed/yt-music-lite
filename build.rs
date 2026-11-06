fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "report_state",
        "get_state",
        "player_action",
        "show_main",
        "start_drag",
        "toggle_maximize",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
