//! Inbox (parked approval requests)

use super::*;

/// Requests waiting on the user (unattended runs park their asks here).
#[tauri::command]
pub fn inbox_pending(state: State<AppState>) -> Vec<crate::core::inbox::Ask> {
    crate::core::inbox::Inbox::load(&state.data_dir).pending()
}

#[tauri::command]
pub fn inbox_all(state: State<AppState>) -> Vec<crate::core::inbox::Ask> {
    crate::core::inbox::Inbox::load(&state.data_dir).list()
}

/// Approve a parked ask — carrying the action out — or deny it. `always` records a
/// standing rule on the automation so the same tool+target won't ask again.
#[tauri::command]
pub async fn inbox_resolve(app: tauri::AppHandle, state: State<'_, AppState>, id: String, approve: bool, always: Option<bool>) -> Result<String, String> {
    let ask = crate::core::inbox::Inbox::load(&state.data_dir).resolve(&id, approve)?;
    crate::core::audit::log(
        &state.data_dir,
        if approve { "approve" } else { "deny" },
        "user",
        &ask.target,
        &ask.summary,
        true,
    );
    if !approve {
        return Ok("Denied — nothing was sent.".into());
    }
    if always.unwrap_or(false) {
        if let Some(task_id) = &ask.task_id {
            let _ = crate::core::automation::AutomationStore::load(&state.data_dir).add_rule(task_id, &ask.tool, &ask.target);
        }
    }
    perform_approved(&app, &state, &ask).await
}
