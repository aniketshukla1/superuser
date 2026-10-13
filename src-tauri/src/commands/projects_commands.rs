//! Projects

use super::*;

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Vec<crate::core::projects::Project> {
    crate::core::projects::list(&state.data_dir)
}

#[tauri::command]
pub fn create_project(state: State<AppState>, name: String, folder: Option<String>) -> Result<crate::core::projects::Project, String> {
    crate::core::projects::create(&state.data_dir, &name, folder)
}

#[tauri::command]
pub fn rename_project(state: State<AppState>, id: String, name: String) {
    crate::core::projects::rename(&state.data_dir, &id, &name)
}

#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) {
    crate::core::projects::delete(&state.data_dir, &id)
}

/// Move a chat into a project (project_id = None removes it from any project).
#[tauri::command]
pub fn set_chat_project(state: State<AppState>, chat_id: String, project_id: Option<String>) {
    crate::core::chats::set_project(&state.data_dir, &chat_id, project_id)
}

/// Read a file off disk and turn it into text or a base64 image for the LLM.
#[tauri::command]
pub fn extract_file(path: String) -> Result<crate::core::files::Extracted, String> {
    crate::core::files::extract(&path)
}

/// Capture a screenshot via the OS and return it as an attachable image. On
/// macOS this uses the built-in interactive `screencapture` (the user drags a
/// region or presses space to grab a window); we hide our own window first so
/// it's not in the shot. Returns Err("cancelled") if the user aborts (Esc).
#[tauri::command]
pub fn capture_screenshot(app: tauri::AppHandle) -> Result<crate::core::files::Extracted, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};
        use tauri::Manager;

        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let tmp = std::env::temp_dir().join(format!("bba-shot-{stamp}.png"));

        // get our window out of the shot, give the compositor a moment, capture
        let main = app.get_webview_window("main");
        if let Some(w) = &main {
            let _ = w.hide();
        }
        std::thread::sleep(std::time::Duration::from_millis(250));

        let status = Command::new("screencapture").arg("-i").arg(&tmp).status();

        if let Some(w) = &main {
            let _ = w.show();
            let _ = w.set_focus();
        }

        match status {
            Ok(s) if s.success() && tmp.exists() => {
                let ex = crate::core::files::extract(tmp.to_string_lossy().as_ref());
                let _ = std::fs::remove_file(&tmp);
                ex
            }
            // screencapture exits 0 even on Esc, but writes no file → treat as cancel
            Ok(_) => Err("cancelled".into()),
            Err(e) => Err(format!("Couldn't run screencapture: {e}")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Screen capture is only available on macOS right now.".into())
    }
}
