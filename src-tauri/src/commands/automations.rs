//! Automations (scheduled assistant runs)

use super::*;

#[tauri::command]
pub fn automations_list(state: State<AppState>) -> Vec<crate::core::automation::ScheduledTask> {
    crate::core::automation::AutomationStore::load(&state.data_dir).list()
}

#[tauri::command]
pub fn automation_create(
    state: State<AppState>,
    title: String,
    instructions: String,
    cron: String,
    persona: Option<String>,
    model: Option<String>,
) -> Result<crate::core::automation::ScheduledTask, String> {
    use crate::core::automation::{next_run_after, AutomationStore, Schedule, ScheduledTask};
    if instructions.trim().is_empty() {
        return Err("instructions are required".into());
    }
    let schedule = Schedule { kind: "cron".into(), cron: Some(cron.trim().to_string()), fire_at: None, timezone: "local".into() };
    if next_run_after(&schedule, 0.0).is_none() {
        return Err("that schedule doesn't produce a next run (use e.g. `0 7 * * *` for 7am daily)".into());
    }
    let workspace = state.data_dir.to_string_lossy().into_owned();
    let task = ScheduledTask::new(title, instructions, schedule, workspace, persona, model);
    Ok(AutomationStore::load(&state.data_dir).create(task))
}

#[tauri::command]
pub fn automation_delete(state: State<AppState>, id: String) -> bool {
    crate::core::automation::AutomationStore::load(&state.data_dir).delete(&id)
}

#[tauri::command]
pub fn automation_set_enabled(state: State<AppState>, id: String, enabled: bool) -> Result<(), String> {
    crate::core::automation::AutomationStore::load(&state.data_dir).set_enabled(&id, enabled)
}

#[tauri::command]
pub fn automation_runs(state: State<AppState>, id: String) -> Vec<crate::core::automation::TaskRun> {
    crate::core::automation::AutomationStore::load(&state.data_dir).list_runs(&id)
}

/// Fire an automation immediately (manual trigger), recording it as a run.
#[tauri::command]
pub async fn automation_run_now(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<AgentRunOut, String> {
    let task = crate::core::automation::AutomationStore::load(&state.data_dir)
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("unknown automation: {id}"))?;
    let run = crate::core::automation::AutomationStore::load(&state.data_dir).start_run(&id, "manual")?;
    // Framed like a scheduled firing, or a manual run of "every morning, summarise my inbox"
    // is read as a request to schedule something.
    let instruction = crate::core::automation::framed(&task.instructions, true);
    let result = run_agent(&app, &state, task.agent.clone(), instruction, Some(run.session_id.clone()), task.model.clone(), None).await;
    let mut store = crate::core::automation::AutomationStore::load(&state.data_dir);
    match &result {
        Ok(out) => { let _ = store.finish_run(&id, &run.run_id, "ok", Some(out.answer.clone()), out.artifacts.clone(), None); }
        Err(e) => { let _ = store.finish_run(&id, &run.run_id, "error", None, vec![], Some(e.clone())); }
    }
    result
}

/// Connect a mailbox over IMAP/SMTP — verified with a real login before storing.
#[tauri::command]
pub async fn email_connect(address: String, app_password: String, imap_host: String, smtp_host: Option<String>) -> Result<String, String> {
    let creds = crate::integrations::email::MailCreds {
        address: address.trim().to_string(),
        password: app_password,
        imap_host: imap_host.trim().to_string(),
        smtp_host: smtp_host.unwrap_or_default().trim().to_string(),
    };
    if creds.address.is_empty() || creds.password.is_empty() || creds.imap_host.is_empty() {
        return Err("address, app password and IMAP host are all required".into());
    }
    let stored = serde_json::json!({
        "address": creds.address,
        "app_password": creds.password,
        "imap_host": creds.imap_host,
        "smtp_host": creds.smtp_host,
    })
    .to_string();
    // IMAP login is blocking.
    let identity = tokio::task::spawn_blocking(move || crate::integrations::email::verify_creds(&creds))
        .await
        .map_err(|e| format!("verify task failed: {e}"))??;
    secrets::set_integration_token("email", &stored);
    secrets::set_account("email", &identity);
    Ok(identity)
}
