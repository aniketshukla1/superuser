use super::super::*;

/// The OS folder picker, as a Tauri command.
///
/// The GUI's `pickFolder()` invokes `pick_folder` directly when running on the
/// desktop; without this command the Browse button silently resolved to null and
/// looked broken. (The HTTP `/v1/workspaces/pick` route is the browser-dev path.)
#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()));
    });
    // The dialog callback lands on another thread; don't block an async worker.
    tokio::task::spawn_blocking(move || rx.recv().ok().flatten()).await.ok().flatten()
}

#[tauri::command]
pub fn disconnect_integration(id: String) -> bool {
    secrets::clear_integration_token(crate::integrations::token_key(&id))
}

#[tauri::command]
pub fn integration_connected(id: String) -> bool {
    secrets::has_integration_token(crate::integrations::token_key(&id))
}

/// Whether the one-time Google OAuth client is configured.
#[tauri::command]
pub fn google_configured() -> bool {
    crate::integrations::google::has_client()
}

/// Run Google sign-in. If client_id/secret are provided they're stored as the
/// app-level client first (one-time setup); otherwise the stored client is used.
/// Returns the signed-in email.
#[tauri::command]
pub async fn google_oauth_start(app: tauri::AppHandle, client_id: Option<String>, client_secret: Option<String>) -> Result<String, String> {
    if let (Some(id), Some(sec)) = (client_id.as_deref(), client_secret.as_deref()) {
        if !id.is_empty() && !sec.is_empty() {
            crate::integrations::google::set_client(id, sec);
        }
    }
    crate::integrations::google::oauth_connect(app).await
}

/// Run a real sync for an integration, writing fresh data into the Memory Tree.
#[tauri::command]
pub async fn sync_integration(state: State<'_, AppState>, id: String) -> Result<u32, String> {
    let vault = state.memory.vault_path.clone();
    let engine = crate::core::memory::MemoryEngine::new(vault);
    let outcome = crate::integrations::sync_one(&id, &engine).await?;

    let norm = crate::integrations::normalize_id(&id).to_string();
    // Refresh the per-source memory count from the vault.
    let total = state.registry.list(&state.memory).into_iter().find(|i| i.id == norm).map(|i| i.memory_count).unwrap_or(outcome.created);
    state.registry.mark_synced(&norm, total);
    Ok(outcome.created)
}
