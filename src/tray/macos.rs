use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use super::Entry;

const ID: &str = "tray";

pub const POPUP_MENU_ITEM: bool = false;

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    let mut builder = MenuBuilder::new(app);
    for entry in super::menu() {
        builder = match entry {
            Entry::Item(id, label) => builder.item(&MenuItem::with_id(app, id, label, true, None::<&str>)?),
            Entry::Separator => builder.item(&PredefinedMenuItem::separator(app)?),
        };
    }

    TrayIconBuilder::with_id(ID)
        .icon(Image::from_bytes(super::ICON)?)
        .tooltip(super::TOOLTIP)
        .menu(&builder.build()?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| super::on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                super::on_click(tray.app_handle(), Some(rect));
            }
        })
        .build(app)?;
    Ok(())
}

pub fn set_icon(app: &AppHandle, png: &[u8], tooltip: &str) {
    let Some(tray) = app.tray_by_id(ID) else {
        return;
    };
    if let Ok(icon) = Image::from_bytes(png) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(tooltip));
}
