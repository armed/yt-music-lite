# YT Music Lite

A minimal desktop wrapper around [music.youtube.com](https://music.youtube.com), built with Tauri 2.
It lives in the tray and does nothing else: no auto-updater, no telemetry, no integrations.

Developed and tested on macOS. On Linux (Wayland) the tray, `toggle`, the floating popup and page load were tried in Sway and Hyprland 0.57 on aarch64 Fedora VMs; audio and sign-in are untested, see [Linux](#linux).

## Features

- Starts hidden in the tray; closing the window hides it instead of quitting.
- Tray popup (left click) with cover, title, artist, seek bar and playback controls.
  Drag it by any empty spot to move it; it opens there from then on.
- Tray menu (right click): open the window, settings, quit.
- Settings window for start-in-tray and the global shortcut.
- Global hotkeys for play/pause, next, previous and showing/hiding the popup.
- Playback commands from the command line.

## Build

Requires the Rust toolchain and [`tauri-cli`](https://tauri.app/reference/cli/).

```sh
cargo install tauri-cli --locked
cargo tauri build --bundles app
```

The app is written to `target/release/bundle/macos/YT Music Lite.app` (about 4 MB).
`just install` builds it and copies it to `/Applications`. On Linux it installs into `~/.local` (binary, `ytm-lite` symlink in `~/.local/bin`, desktop entry and icon); `just uninstall` removes them.
For development, use `cargo run`.

Sign in to your Google account once from the main window.

## Settings

`~/Library/Application Support/dev.armed.ytm-lite/settings.json`, created on first launch:

```json
{
  "start_hidden": true,
  "shortcuts": {
    "toggle_popup": "CmdOrCtrl+Alt+M"
  }
}
```

The settings window edits this file; the shortcut applies immediately. It uses the [accelerator syntax](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/struct.HotKey.html).

## Command line

```sh
ytm-lite play-pause
ytm-lite next
ytm-lite prev
ytm-lite toggle   # show/hide the mini player
```

Enable "Command line tool" in the settings window to link `ytm-lite` into `~/.local/bin`
(make sure it is on your `PATH`). If the app is running, the command is sent to it and the process exits immediately. Otherwise the
app is started and the action is applied once a track has loaded.

## Linux

Build with `cargo tauri build --bundles deb,rpm` or just `cargo build --release`.
Needs WebKitGTK 4.1 and libayatana-appindicator (or libappindicator-gtk3) at build time,
and the GStreamer `good`, `bad` and `libav` plugins at run time for audio playback
(on Fedora the AAC codecs come from RPM Fusion).
The data directory is `$XDG_CONFIG_HOME/dev.armed.ytm-lite` (default `~/.config`).

The tray icon does not report clicks on Linux, so the mini player is opened from the
tray menu ("Mini player") or with `ytm-lite toggle`. Global shortcuts do not work on
Wayland: bind the commands in your compositor instead. The tray needs a bar that
hosts StatusNotifier icons (waybar `tray` module, KDE, GNOME with the AppIndicator extension).

On Wayland the compositor decides where the mini player goes. On KDE Plasma the app adds a KWin
window rule "YT Music Popup" on first start: bottom-right corner, kept above, hidden from the taskbar
and the window switcher, and KWin remembers where it was dragged to. Change it in System Settings →
Window Management → Window Rules; the app does not touch it once it exists. Other compositors need
a rule that makes it floating. Sway:

```
bindsym $mod+m exec ytm-lite toggle
bindsym XF86AudioPlay exec ytm-lite play-pause
bindsym XF86AudioNext exec ytm-lite next
bindsym XF86AudioPrev exec ytm-lite prev
for_window [title="YT Music Popup"] floating enable, sticky enable, move position 1400 40
```

Hyprland 0.57 and newer read a Lua config, `~/.config/hypr/hyprland.lua`:

```lua
hl.bind("SUPER + P", hl.dsp.exec_cmd("ytm-lite toggle"))
hl.window_rule({
    name  = "ytm-popup",
    match = { title = "^(YT Music Popup)$" },
    float = true,
    pin   = true,
    move  = "100 40",
})
hl.on("hyprland.start", function ()
  hl.exec_cmd("waybar")
  hl.exec_cmd("ytm-lite")
end)
```

The rule and the autostart were run on Hyprland 0.57.2; the key binding follows the form in the generated
default config. The syntax differs on older Hyprland versions, check its wiki for yours.

If the mini player draws as an empty transparent box, WebKitGTK cannot create a GL context
(seen in a VM with virgl). Start the app with `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
