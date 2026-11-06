# YT Music Lite

A minimal desktop wrapper around [music.youtube.com](https://music.youtube.com), built with Tauri 2.
It lives in the menu bar / system tray and does nothing else: no auto-updater, no telemetry,
no Discord / Last.fm / lyrics integrations.

Developed and tested on macOS. Windows and Linux are not tested.

## Features

- Starts hidden in the tray (configurable); closing the window hides it instead of quitting.
- Left click on the tray icon opens a compact player popup: cover, title, artist, seek bar,
  like / dislike, previous / play-pause / next, mute and "open window".
- Right click on the tray icon opens a menu: open the main window, settings, quit.
- Global hotkeys for play/pause, next and previous.
- Clicking the Dock icon brings the main window back.
- The tray icon pulses while the page is still loading.
- macOS: overlay title bar, drag the window by any empty area, no text selection outside inputs,
  hidden scrollbars.

## Build and install

Requirements: Rust toolchain and [`tauri-cli`](https://tauri.app/reference/cli/).

```sh
cargo install tauri-cli --locked
cargo tauri build --bundles app
```

The bundle is written to `target/release/bundle/macos/YT Music Lite.app` (about 4 MB); copy it to
`/Applications`. The release profile is tuned for size (`opt-level = "z"`, LTO, stripped), so a
release build takes a while.

For development:

```sh
cargo run
```

Sign in to your Google account once from the main window. The session is stored in a WebKit data
store with a fixed identifier, so it survives rebuilds and reinstalls.

## Settings

Settings live in a JSON file that is created on first launch:

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/dev.armed.ytmusic-rust/settings.json` |
| Linux | `~/.config/dev.armed.ytmusic-rust/settings.json` |

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

`start_hidden` can also be toggled from the tray menu (Settings → Start hidden in tray).
Shortcuts use the [global-hotkey accelerator syntax](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/struct.HotKey.html)
and are read at startup, so restart the app after editing them. The tray menu has
"Open settings file…" for convenience.

## Command line

The binary accepts playback flags, which makes it scriptable from other tools:

```sh
ytmusic-rust --play-pause
ytmusic-rust --next
ytmusic-rust --prev
```

If the app is already running, the command is sent to it over a local Unix socket
(`control.sock` next to `settings.json`) and the process exits immediately. If the app is not
running, it is started (through LaunchServices on macOS) and the action is applied as soon as the
player has loaded a track.

## Layout

| Path | Purpose |
|---|---|
| `src/main.rs` | Windows, tray, hotkeys, control socket, IPC commands |
| `src/settings.rs` | Settings file |
| `src/init.js` | Script injected into music.youtube.com: player controls, state reporting, window dragging, styling |
| `dist/index.html` | Tray popup UI |
| `capabilities/`, `build.rs` | Tauri permissions; the remote page may only call the commands it needs |
