//! Desktop behaviour the GUI has always asked for


#[tauri::command]
pub fn get_autostart() -> bool {
    crate::core::system::autostart_enabled()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<bool, String> {
    crate::core::system::set_autostart(enabled)
}

#[tauri::command]
pub fn get_keep_awake() -> bool {
    crate::core::system::keep_awake_enabled()
}

#[tauri::command]
pub fn set_keep_awake(enabled: bool) -> Result<bool, String> {
    crate::core::system::set_keep_awake(enabled)
}

/// Drag the window by its own chrome.
///
/// The app draws its own title bar (`titleBarStyle: Overlay`), so without this the
/// only draggable part of the window was the small strip the system still owns.
#[tauri::command]
pub fn start_window_drag(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

/// Is there a newer version?
///
/// This build has no update channel, and saying so is the point: the command
/// returned null before, which the banner read as "you are up to date" — the same
/// answer a working check gives when there is nothing new. A user would never learn
/// that updates had never been checked at all.
#[tauri::command]
pub fn check_for_update() -> crate::core::system::UpdateStatus {
    crate::core::system::UpdateStatus::none(
        "This build has no update channel configured, so it cannot check for updates.",
    )
}

#[tauri::command]
pub fn download_update() -> Result<(), String> {
    Err("This build has no update channel configured — update it the way you installed it.".into())
}

#[tauri::command]
pub fn install_update() -> Result<(), String> {
    Err("This build has no update channel configured — update it the way you installed it.".into())
}

#[tauri::command]
pub fn clear_pending_update() {}
