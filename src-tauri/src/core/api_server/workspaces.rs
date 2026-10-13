//! workspaces

use super::*;

// A "workspace" is the folder a code-family assistant works in. Knowledge personas
// use their session's own working folder, so this stays deliberately small.

pub(super) async fn workspaces_recent(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let items: Vec<Value> = crate::core::projects::list(&state.data_dir)
        .into_iter()
        .map(|p| json!({
            "path": p.folder.clone().unwrap_or_default(),
            "name": p.name,
            // Was `created_at` under this name — so a folder opened daily for a month
            // still reported the day it was first seen, and the GUI had no way to know.
            "last_opened": p.last_opened.unwrap_or(p.created_at),
            // The GUI greys out folders that have since been moved or deleted.
            "exists": p.folder.as_deref().map(|f| std::path::Path::new(f).is_dir()).unwrap_or(false),
        }))
        .collect();
    Json(json!({ "workspaces": items }))
}

pub(super) async fn workspace_open(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let path = body["path"].as_str().unwrap_or("").to_string();
    if path.trim().is_empty() {
        return Json(json!({ "ok": false, "path": path, "error": "no path given" }));
    }
    if !std::path::Path::new(&path).is_dir() {
        return Json(json!({ "ok": false, "path": path, "error": "that folder doesn't exist" }));
    }
    let state = ctx.app.state::<crate::AppState>();
    let name = std::path::Path::new(&path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
    let _ = crate::core::projects::create(&state.data_dir, &name, Some(path.clone()));
    Json(json!({ "ok": true, "path": path }))
}

/// The OS folder picker — a web dialog can't return an absolute path.
pub(super) async fn workspace_pick(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()));
    });
    let picked = tokio::task::spawn_blocking(move || rx.recv().ok().flatten()).await.ok().flatten();
    Json(json!({ "path": picked }))
}
