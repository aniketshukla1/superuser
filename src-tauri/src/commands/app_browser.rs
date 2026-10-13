//! The in-app browser (a child webview; see core/browser.rs)


/// Create-or-navigate the browser webview and place it at the tab's bounds.
/// Bounds are logical (CSS) pixels measured by the frontend.
#[tauri::command]
pub fn browser_open(app: tauri::AppHandle, url: String, x: f64, y: f64, w: f64, h: f64) -> Result<String, String> {
    crate::core::browser::open(&app, &url, x, y, w, h)
}

#[tauri::command]
pub fn browser_navigate(app: tauri::AppHandle, url: String) -> Result<String, String> {
    crate::core::browser::navigate(&app, &url)
}

/// Track the panel through window resizes and sidebar collapses.
#[tauri::command]
pub fn browser_bounds(app: tauri::AppHandle, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    crate::core::browser::set_bounds(&app, x, y, w, h)
}

/// A child webview is a native view above the HTML, so it cannot be covered by a
/// modal or a dropdown — it has to be hidden. The page stays alive.
#[tauri::command]
pub fn browser_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    crate::core::browser::set_visible(&app, visible)
}

#[tauri::command]
pub fn browser_history(app: tauri::AppHandle, action: String) -> Result<(), String> {
    crate::core::browser::history(&app, &action)
}

#[tauri::command]
pub fn browser_close(app: tauri::AppHandle) -> Result<(), String> {
    crate::core::browser::close(&app)
}

#[tauri::command]
pub fn browser_url(app: tauri::AppHandle) -> Option<String> {
    crate::core::browser::current_url(&app)
}

/// Dev servers the workspace declares, for the browser's start page.
#[tauri::command]
pub fn browser_dev_servers(workspace: String) -> Vec<crate::core::browser::DevServer> {
    crate::core::browser::dev_servers(std::path::Path::new(&workspace))
}

/// Start a dev server in its OWN pty rooted at `cwd` — not in the shared shell,
/// which is rooted at the session's workspace and would run it in the wrong folder.
#[tauri::command]
pub fn devserver_start(app: tauri::AppHandle, name: String, cwd: String, command: String) -> Result<(), String> {
    crate::core::devserver::start(&app, &name, &cwd, &command)
}

#[tauri::command]
pub fn devserver_stop(name: String) {
    crate::core::devserver::stop(&name);
}

#[tauri::command]
pub fn devserver_running() -> Vec<String> {
    crate::core::devserver::running()
}
