// Tauri's Linux tray goes through libappindicator, which marks the item as a
// menu and never reports clicks, so the StatusNotifierItem is served directly.

use std::sync::Mutex;

use ksni::{blocking::TrayMethods, menu::StandardItem, Icon, MenuItem, ToolTip};
use tauri::{image::Image, AppHandle, Manager};

use super::Entry;

// Not every StatusNotifierItem host sends Activate on a left click.
pub const POPUP_MENU_ITEM: bool = true;

struct Tray {
    app: AppHandle,
    icon: Option<Icon>,
    tooltip: String,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn title(&self) -> String {
        super::TOOLTIP.into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.iter().cloned().collect()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: self.tooltip.clone(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        super::on_click(&self.app, None);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        super::menu()
            .into_iter()
            .map(|entry| match entry {
                Entry::Item(id, label) => StandardItem {
                    label: label.into(),
                    activate: Box::new(move |tray: &mut Self| super::on_menu(&tray.app, id)),
                    ..Default::default()
                }
                .into(),
                Entry::Separator => MenuItem::Separator,
            })
            .collect()
    }
}

struct Handle(Mutex<ksni::blocking::Handle<Tray>>);

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    let tray = Tray {
        app: app.handle().clone(),
        icon: pixmap(super::ICON),
        tooltip: super::TOOLTIP.into(),
    };
    // The panel may come up after the app when both are autostarted.
    match tray.assume_sni_available(true).spawn() {
        Ok(handle) => {
            app.manage(Handle(Mutex::new(handle)));
        }
        Err(err) => eprintln!("cannot show the tray icon: {err}"),
    }
    Ok(())
}

pub fn set_icon(app: &AppHandle, png: &[u8], tooltip: &str) {
    let Some(handle) = app.try_state::<Handle>() else {
        return;
    };
    let icon = pixmap(png);
    handle.0.lock().unwrap().update(|tray| {
        tray.icon = icon;
        tray.tooltip = tooltip.into();
    });
}

fn pixmap(png: &[u8]) -> Option<Icon> {
    let image = Image::from_bytes(png).ok()?;
    let (pixels, _) = image.rgba().as_chunks::<4>();
    let data = pixels.iter().flat_map(|&[r, g, b, a]| [a, r, g, b]).collect();
    Some(Icon {
        width: image.width() as i32,
        height: image.height() as i32,
        data,
    })
}
