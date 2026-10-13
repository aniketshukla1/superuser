use super::super::*;

/// Create a scheduled automation. `permissions` are the standing grants the
/// creating surface displayed — submitting IS the consent — but only target-bound
/// write entries survive validation.
pub(in crate::core::api_server) async fn automation_create(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use crate::core::automation::{next_run_after, AutomationStore, Schedule, ScheduledTask};
    use tauri::Manager;
    let title = body["title"].as_str().unwrap_or("").trim().to_string();
    let instructions = body["instructions"].as_str().unwrap_or("").trim().to_string();
    if instructions.is_empty() {
        return Json(json!({ "ok": false, "error": "instructions are required" }));
    }
    let cron = body["cron"].as_str().unwrap_or("").trim().to_string();
    let fire_at = body["fire_at"].as_str().map(|s| s.to_string());
    let schedule = if !cron.is_empty() {
        Schedule { kind: "cron".into(), cron: Some(cron), fire_at: None, timezone: body["timezone"].as_str().unwrap_or("local").into() }
    } else if let Some(at) = fire_at {
        // Pinned to the zone it was written in, so it fires at that time there (F-6).
        let timezone = body["timezone"].as_str().unwrap_or("local").to_string();
        let fire_at = Some(crate::core::automation::pin_fire_at(&at, &timezone));
        Schedule { kind: "once".into(), cron: None, fire_at, timezone }
    } else {
        return Json(json!({ "ok": false, "error": "give a cron expression or a fire_at time" }));
    };
    if next_run_after(&schedule, 0.0).is_none() {
        return Json(json!({ "ok": false, "error": "that schedule has no next run (e.g. `0 7 * * *` for 7am daily)" }));
    }

    let state = ctx.app.state::<crate::AppState>();
    let dir = state.data_dir.clone();
    let workspace = dir.to_string_lossy().into_owned();
    let mut task = ScheduledTask::new(
        if title.is_empty() { instructions.chars().take(50).collect() } else { title },
        instructions,
        schedule,
        workspace,
        body["agent"].as_str().map(String::from),
        body["model"].as_str().map(String::from),
    );
    // Remember which conversation this was set up from, so each run's answer comes
    // back to it. Absent for automations created outside a chat — those just don't
    // inject anywhere.
    task.origin_session = body["session_id"].as_str().filter(|s| !s.is_empty()).map(String::from);
    // Only target-bound write grants are stored; reads are disclosure-only.
    for perm in body["permissions"].as_array().cloned().unwrap_or_default() {
        if perm["access"].as_str() == Some("write") {
            let (tool, target) = (perm["tool"].as_str().unwrap_or(""), perm["target"].as_str().unwrap_or(""));
            if !tool.is_empty() && !target.is_empty() {
                task.add_rule(tool, target);
            }
        }
    }
    let created = AutomationStore::load(&dir).create(task);
    Json(json!({
        "ok": true,
        "task": {
            "id": created.id, "title": created.title, "instructions": created.instructions,
            "schedule": created.human_schedule(), "schedule_raw": created.schedule,
            "workspace": created.workspace, "agent": created.agent, "enabled": created.enabled,
            "next_run": created.next_run, "last_run": created.last_run, "last_status": created.last_status,
            "run_count": created.run_count, "notify_on_completion": created.notify_on_completion,
            "model": created.model,
        }
    }))
}

/// Enable/disable or delete an automation.
pub(in crate::core::api_server) async fn automation_patch(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let mut store = crate::core::automation::AutomationStore::load(&dir);
    // `enabled` was the only field this accepted, which meant an automation pointed
    // at the wrong folder could be turned off and deleted but never corrected. A
    // task whose workspace has gone stale still runs — it just can't find anything.
    let mut changed = false;
    let mut out = json!({ "ok": true });
    if let Some(v) = body["enabled"].as_bool() {
        if let Err(e) = store.set_enabled(&id, v) {
            return Json(json!({ "ok": false, "error": e }));
        }
        changed = true;
    }
    if let Some(w) = body["workspace"].as_str().map(str::trim).filter(|w| !w.is_empty()) {
        match store.set_workspace(&id, w) {
            Ok(path) => {
                out["workspace"] = json!(path);
                changed = true;
            }
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    // `null` is meaningful here and `""` is the same request: put this automation back
    // on the app's default. Only an absent key means "leave the model alone", which is
    // what a PATCH that is not about the model looks like.
    // Title, wording and schedule — the three the detail view has always offered and
    // none of which this route accepted, so every save answered "nothing to update".
    let (title, instructions, cron) = (
        body["title"].as_str(),
        body["instructions"].as_str(),
        body["cron"].as_str(),
    );
    if title.is_some() || instructions.is_some() || cron.is_some() {
        if let Err(e) = store.edit(&id, title, instructions, cron) {
            return Json(json!({ "ok": false, "error": e }));
        }
        crate::trace!("automation", "edited id={id} title={:?} cron={:?}", title, cron);
        changed = true;
    }
    // A standing approval being handed back. Also sent by the GUI and also ignored:
    // the revoke button reported success and the rule stayed.
    if let Some(entry) = body["revoke"].as_str().map(str::trim).filter(|e| !e.is_empty()) {
        match store.revoke_rule(&id, entry) {
            Ok(removed) => {
                crate::trace!("automation", "revoke id={id} entry={entry:?} removed={removed}");
                out["revoked"] = json!(removed);
                changed = true;
            }
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    if body.get("model").is_some() {
        match store.set_model(&id, body["model"].as_str()) {
            Ok(m) => {
                crate::trace!("automation", "model set id={id} model={:?}", m);
                out["model"] = json!(m);
                changed = true;
            }
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    if changed {
        Json(out)
    } else {
        Json(json!({ "ok": false, "error": "nothing to update" }))
    }
}

pub(in crate::core::api_server) async fn automation_delete(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({ "ok": crate::core::automation::AutomationStore::load(&dir).delete(&id) }))
}

/// Fire an automation now, through the same path the scheduler uses.
/// Prepare a manual run: open its session, hand back what the GUI needs to drive it.
///
/// It used to EXECUTE here — one HTTP request that ran a whole agent turn and
/// returned when it was done. Three things wrong with that. The request blocks an
/// axum handler for minutes (this task's own history has a 14,726-second run in
/// it); nothing streams, so "Run now" sat dead with no steps and no way to stop;
/// and the caller has always expected the opposite contract — `runTaskNow` reads
/// `prompt`, `workspace` and `agent` off the reply, and none of them were ever
/// sent, so the session it opened afterwards had no prompt to send.
///
/// So this starts the run record and returns the handles. The GUI opens that
/// session, sends the prompt, watches it like any other turn, and calls
/// `/finalize` when the turn ends.
pub(in crate::core::api_server) async fn automation_run(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let Some(task) = crate::core::automation::AutomationStore::load(&dir).get(&id).cloned() else {
        return Json(json!({ "ok": false, "error": format!("unknown automation: {id}") }));
    };
    let mut store = crate::core::automation::AutomationStore::load(&dir);
    let run = match store.start_run(&id, "manual") {
        Ok(r) => r,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    Json(json!({
        "ok": true,
        "run_id": run.run_id,
        "session_id": run.session_id,
        "workspace": task.workspace,
        "agent": task.agent,
        "prompt": task.instructions,
    }))
}
