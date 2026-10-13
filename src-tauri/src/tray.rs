//! The menu-bar icon and the app menu.
//!
//! The app keeps working with its window closed: routines fire, the Slack and Telegram
//! watchers answer, the local API serves. Something on screen has to say it is still
//! running, and offer the way back in and the way out; the mascot does the first only while
//! it is shown, and says nothing about quitting.
//!
//! One handler serves both menus. Tauri hands every menu event — the tray's and the app
//! menu's — to the app-wide listeners, so a second handler on the tray would run each click
//! twice.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::Manager;

const OPEN: &str = "su-open";
const SETTINGS: &str = "su-settings";
const QUIT: &str = "su-quit";

/// The event the GUI already listens for to open Settings (`App.tsx`).
const OPEN_SETTINGS_JS: &str = "window.dispatchEvent(new Event('assistant:open-settings'))";

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();

    // The app menu, on macOS only: the usual one, with Settings… where a Mac person looks for
    // it. Elsewhere the default menu is a bar across the window that isn't there today.
    #[cfg(target_os = "macos")]
    {
        let menu = Menu::default(handle)?;
        if let Some(first) = menu.items()?.first().and_then(|i| i.as_submenu().cloned()) {
            let settings = MenuItem::with_id(handle, SETTINGS, "Settings…", true, Some("CmdOrCtrl+,"))?;
            // After "About", before the separator that follows it.
            first.insert(&settings, 1)?;
        }
        app.set_menu(menu)?;
    }

    let tray_menu = Menu::with_items(
        handle,
        &[
            &MenuItem::with_id(handle, OPEN, "Open Superuser", true, None::<&str>)?,
            &MenuItem::with_id(handle, SETTINGS, "Settings…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(handle)?,
            // Says what quitting stops, because closing the window doesn't.
            &MenuItem::with_id(handle, QUIT, "Quit — routines and watchers stop", true, None::<&str>)?,
        ],
    )?;
    let mut tray = tauri::tray::TrayIconBuilder::with_id("superuser")
        .tooltip("Superuser — keeps working while its window is closed")
        .menu(&tray_menu)
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }
    tray.build(app)?;

    app.on_menu_event(|app, event| match event.id().as_ref() {
        OPEN => crate::commands::show_main_window(app.clone()),
        SETTINGS => {
            crate::commands::show_main_window(app.clone());
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval(OPEN_SETTINGS_JS);
            }
        }
        QUIT => app.exit(0),
        _ => {}
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn settings_opens_through_the_event_the_gui_listens_for() {
        let Ok(src) = crate::test_support::gui_source("App.tsx") else { return };
        assert!(
            src.contains("addEventListener(\"assistant:open-settings\""),
            "the tray's Settings item dispatches an event nothing listens for"
        );
        assert!(super::OPEN_SETTINGS_JS.contains("assistant:open-settings"));
    }
}
