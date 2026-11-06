# YT Music Lite

A minimal desktop wrapper around [music.youtube.com](https://music.youtube.com), built with Tauri 2.
It lives in the tray and does nothing else: no auto-updater, no telemetry, no integrations.

Developed and tested on macOS only.

## Features

- Starts hidden in the tray; closing the window hides it instead of quitting.
- Tray popup (left click) with cover, title, artist, seek bar and playback controls.
- Tray menu (right click): open the window, settings, quit.
- Global hotkeys for play/pause, next and previous.
- Playback commands from the command line.

## Build

Requires the Rust toolchain and [`tauri-cli`](https://tauri.app/reference/cli/).

```sh
cargo install tauri-cli --locked
cargo tauri build --bundles app
```

The app is written to `target/release/bundle/macos/YT Music Lite.app` (about 4 MB).
For development, use `cargo run`.

Sign in to your Google account once from the main window.

## Settings

`~/Library/Application Support/dev.armed.ytm-lite/settings.json`, created on first launch:

```json
{
  "start_hidden": true,
  "shortcuts": {
    "play_pause": "CmdOrCtrl+Alt+P",
    "next": "CmdOrCtrl+Alt+Right",
    "prev": "CmdOrCtrl+Alt+Left"
  }
}
```

Shortcuts use the [accelerator syntax](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/struct.HotKey.html)
and are read at startup.

## Command line

```sh
ytm-lite --play-pause
ytm-lite --next
ytm-lite --prev
```

If the app is running, the command is sent to it and the process exits immediately. Otherwise the
app is started and the action is applied once a track has loaded.
