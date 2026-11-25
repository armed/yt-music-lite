app := "YT Music Lite.app"
id := "dev.armed.ytm-lite"
share := env("XDG_DATA_HOME", home_directory() / ".local/share")
libdir := home_directory() / ".local/lib/ytm-lite"
bindir := home_directory() / ".local/bin"

[macos]
install:
    cargo tauri build --bundles app
    rm -rf "/Applications/{{app}}"
    ditto "target/release/bundle/macos/{{app}}" "/Applications/{{app}}"

# The binary lives outside ~/.local/bin so the symlink there matches what the
# "Command line tool" setting creates and that setting shows as enabled.
[linux]
install:
    cargo build --release
    install -Dm755 target/release/ytm-lite "{{libdir}}/ytm-lite"
    mkdir -p "{{bindir}}"
    ln -sf "{{libdir}}/ytm-lite" "{{bindir}}/ytm-lite"
    install -Dm644 icons/icon.png "{{share}}/icons/hicolor/512x512/apps/{{id}}.png"
    mkdir -p "{{share}}/applications"
    printf '%s\n' \
        '[Desktop Entry]' \
        'Type=Application' \
        'Name=YT Music Lite' \
        'Exec={{libdir}}/ytm-lite' \
        'Icon={{id}}' \
        'Categories=AudioVideo;Audio;Music;' \
        'StartupWMClass=ytm-lite' \
        'Terminal=false' \
        > "{{share}}/applications/{{id}}.desktop"
    -update-desktop-database "{{share}}/applications"

[linux]
uninstall:
    rm -f "{{bindir}}/ytm-lite" "{{share}}/applications/{{id}}.desktop" "{{share}}/icons/hicolor/512x512/apps/{{id}}.png"
    rm -rf "{{libdir}}"
