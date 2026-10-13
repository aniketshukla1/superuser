//! Audit log

use super::*;

/// The most recent audited actions (newest first) — every send, sync, connect,
/// desktop action and approval this app performed.
#[tauri::command]
pub fn audit_recent(state: State<AppState>, limit: Option<usize>) -> Vec<crate::core::audit::Entry> {
    crate::core::audit::recent(&state.data_dir, limit.unwrap_or(200))
}

#[tauri::command]
pub fn audit_clear(state: State<AppState>) -> bool {
    crate::core::audit::clear(&state.data_dir)
}

/// Connect Slack with a bot token: verified against auth.test before storing.
/// Once connected, mentioning the bot in a channel starts an assistant session.
#[tauri::command]
pub async fn slack_connect(state: State<'_, AppState>, token: String) -> Result<String, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("paste the bot token (xoxb-…)".into());
    }
    let identity = crate::integrations::slack::verify(&token).await?;
    secrets::set_integration_token("slack", &token);
    secrets::set_account("slack", &identity);
    // This instance's own trail: in a scratch instance the real app's folder is another
    // process's hash chain, and a chain written by two processes breaks.
    crate::core::audit::log(&state.data_dir, "connect", "user", "slack", &identity, true);
    Ok(identity)
}
