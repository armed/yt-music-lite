app := "YT Music Lite.app"

install:
    cargo tauri build --bundles app
    rm -rf "/Applications/{{app}}"
    ditto "target/release/bundle/macos/{{app}}" "/Applications/{{app}}"
