use super::super::*;

/// `POST /v1/watches/tick` — run a pass now instead of waiting for the interval.
/// Blocking, deliberately: the caller wants the result, and a pass is short.
pub(in crate::core::api_server) async fn watches_tick(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let (settings, dir, vault) = {
        let state = ctx.app.state::<crate::AppState>();
        let s = state.settings.lock().unwrap().clone();
        (s, state.data_dir.clone(), state.memory.vault_path.clone())
    };
    let memory = crate::core::memory::MemoryEngine::new(vault);
    let r = crate::core::subconscious::tick(&settings, &dir, &memory).await;
    Json(json!({
        "ok": true,
        "evaluated": r.evaluated, "quiet": r.skipped_quiet,
        "acted": r.acted, "escalated": r.escalated, "failed": r.failed,
    }))
}

/// One automation, in the shape both the list and the detail page read.
///
/// Shared rather than written twice: the two views disagreeing about a field is
/// how a badge ends up on a card that its own detail page says nothing about.
pub(in crate::core::api_server) fn task_json(store: &crate::core::automation::AutomationStore, t: &crate::core::automation::ScheduledTask) -> Value {
    let (unseen_runs, unseen_failed) = store.unseen(&t.id);
    let mut v = json!({
                "id": t.id,
                "title": t.title,
                "instructions": t.instructions,
                // `schedule` is the human label; the raw form rides alongside.
                "schedule": t.human_schedule(),
                "schedule_raw": t.schedule,
                "workspace": t.workspace,
                "agent": t.agent,
                // Null means "whatever the app would pick", which the GUI renders as
                // Default. It was never sent at all, so an automation pinned to a model
                // looked exactly like one that was not.
                "model": t.model,
                "enabled": t.enabled,
                "next_run": t.next_run,
                "last_run": t.last_run,
                "last_status": t.last_status,
                "run_count": t.run_count,
        "notify_on_completion": t.notify_on_completion,
        "task_session_id": t.task_session_id(),
    });
    // The unread mark. The GUI has always read these two; nothing ever sent them,
    // so every automation looked equally read whether it had run overnight or not.
    v["unseen_runs"] = json!(unseen_runs);
    v["unseen_failed"] = json!(unseen_failed);
    v
}

pub(in crate::core::api_server) async fn automations_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let store = crate::core::automation::AutomationStore::load(&dir);
    let tasks: Vec<Value> = store.list().iter().map(|t| task_json(&store, t)).collect();
    // The GUI reads `.tasks` here (not `.automations`).
    Json(json!({ "tasks": tasks }))
}

/// One automation and its run history — the detail page.
///
/// This route did not exist. `/v1/automations/:id` was registered for PATCH and
/// DELETE only, so opening a task answered 405 and the page rendered an automation
/// with no runs, for a scheduler that had in fact run it nineteen times.
pub(in crate::core::api_server) async fn automation_get(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let store = crate::core::automation::AutomationStore::load(&dir);
    let Some(task) = store.get(&id) else {
        return Json(json!({ "ok": false, "error": format!("unknown automation: {id}") }));
    };
    // Newest first: the run you want is almost always the last one.
    let mut runs = store.list_runs(&id);
    runs.sort_by(|a, b| b.started_at.total_cmp(&a.started_at));
    // Does this run have a transcript to open?
    //
    // Until now a scheduled run's session was a working folder with no
    // `session.json` in it, so every "open this run's conversation" landed on an
    // empty chat. The scheduler records the conversation now, but the runs already
    // on disk never will — so say which ones can be opened rather than offering the
    // door and letting it lead nowhere.
    let sessions = crate::core::sessions::SessionStore::new(&dir);
    let runs: Vec<Value> = runs
        .into_iter()
        .map(|r| {
            let has = sessions.get(&r.session_id).map(|s| !s.messages.is_empty()).unwrap_or(false);
            let mut v = serde_json::to_value(&r).unwrap_or_else(|_| json!({}));
            v["has_conversation"] = json!(has);
            v
        })
        .collect();
    Json(json!({ "ok": true, "task": task_json(&store, task), "runs": runs }))
}

/// Clear a task's unread-runs mark.
pub(in crate::core::api_server) async fn automation_seen(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let mut store = crate::core::automation::AutomationStore::load(&dir);
    match store.mark_seen(&id) {
        Ok(()) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// Close out a manual run once the session it opened has finished its turn.
///
/// The answer is read back from that session rather than posted by the client: the
/// run record should say what the assistant actually produced, and the client would
/// be reporting on itself.
pub(in crate::core::api_server) async fn automation_finalize(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path((id, run_id)): axum::extract::Path<(String, String)>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let mut store = crate::core::automation::AutomationStore::load(&dir);
    let session = store.list_runs(&id).into_iter().find(|r| r.run_id == run_id).map(|r| r.session_id);
    let answer = session.and_then(|sid| {
        crate::core::sessions::SessionStore::new(&dir)
            .get(&sid)
            .and_then(|s| s.messages.iter().rev().find(|m| m.role == "assistant").map(|m| m.content.clone()))
    });
    match store.finish_run(&id, &run_id, "ok", answer, vec![], None) {
        Ok(()) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// The first question's option labels, for the Inbox's simpler card.
pub(in crate::core::api_server) fn inbox_options(args: &Value) -> Vec<String> {
    let first = args["questions"].as_array().and_then(|q| q.first()).cloned().unwrap_or_else(|| args.clone());
    first["options"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|o| o["label"].as_str().or_else(|| o.as_str()))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Parked asks, mapped into the GUI's InboxItem shape.
/// What `GET /v1/inbox` may be narrowed by.
///
/// Both fields have been sent by the GUI since it was written and neither was read.
/// `getInbox(sessionId, "pending")` asked for one conversation's open questions and
/// got every item in the inbox, from every session, resolved ones included — so a
/// question parked by an unattended automation run three hours ago, and already
/// denied, rendered as a live card in someone's chat. Clicking it posted a
/// resolution for an item that was already resolved, the server ignored it, and the
/// card sat there doing nothing.
#[derive(serde::Deserialize, Default)]
pub(in crate::core::api_server) struct InboxQuery {
    #[serde(default)]
    pub(in crate::core::api_server) session_id: String,
    /// `pending` | `resolved`. Anything else, or absent, means no filter.
    #[serde(default)]
    pub(in crate::core::api_server) state: String,
}

/// Whether one item belongs in a filtered listing. Separate from the handler so the
/// filter is testable without a server.
pub(in crate::core::api_server) fn inbox_matches(a: &crate::core::inbox::Ask, q: &InboxQuery) -> bool {
    if !q.session_id.is_empty() && a.session_id != q.session_id {
        return false;
    }
    match q.state.as_str() {
        // The record's own vocabulary is `pending | approved | denied`; the API's is
        // `pending | resolved`. Mapping here rather than at the caller is what stops
        // a client having to know both.
        "pending" => a.status == "pending",
        "resolved" => a.status != "pending",
        _ => true,
    }
}
