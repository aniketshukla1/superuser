use super::super::*;

/// Approve or deny a parked ask; approving a send delivers it now.
pub(in crate::core::api_server) async fn inbox_resolve_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    // "once"/"approve"/"always_*" all mean yes; anything else is a denial.
    let resolution = body["resolution"].as_str().unwrap_or("deny");
    let approve = matches!(resolution, "once" | "approve" | "approved") || resolution.starts_with("always");
    let ask = match crate::core::inbox::Inbox::load(&dir).resolve(&id, approve) {
        Ok(a) => a,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    crate::core::audit::log(&dir, if approve { "approve" } else { "deny" }, "user", &ask.target, &ask.summary, true);
    if approve && resolution.starts_with("always") {
        if let Some(task_id) = &ask.task_id {
            let _ = crate::core::automation::AutomationStore::load(&dir).add_rule(task_id, &ask.tool, &ask.target);
        }
    }
    if !approve {
        // The agent that asked has long since moved on — it was told "parked" and
        // carried on. If its turn is still running, tell it the answer so it stops
        // waiting on something that is never coming.
        tell_the_asker(&ask, "was denied by the user. Don't retry it; work around it or say why you can't.");
        return Json(json!({ "ok": true, "resolution": "denied" }));
    }

    // Approving *performs* the action. There is no suspended step to resume: the
    // call returned "parked" when it was gated. See `perform_approved`.
    let state = ctx.app.state::<crate::AppState>();
    match crate::commands::perform_approved(&ctx.app, &state, &ask).await {
        Ok(detail) => {
            tell_the_asker(&ask, &format!("was approved and has now been carried out: {detail}"));
            Json(json!({ "ok": true, "resolution": "approved", "detail": detail }))
        }
        Err(e) => {
            // Approved but failed is its own outcome, and the audit line above
            // already claimed success — correct the record rather than leave it.
            crate::core::audit::log(&dir, "approve", "user", &ask.target, &format!("failed: {e}"), false);
            Json(json!({ "ok": false, "error": e }))
        }
    }
}

/// Let a still-running turn know how its parked ask was decided.
///
/// Reuses the steer queue: the outcome arrives at the agent's next step boundary,
/// exactly like something the user typed. A turn that has already finished simply
/// has nothing to tell — the transcript and the Inbox both carry the decision.
pub(in crate::core::api_server) fn tell_the_asker(ask: &crate::core::inbox::Ask, outcome: &str) {
    if is_running(&ask.session_id) {
        queue_steer(&ask.session_id, &format!("Your request to {} on {} {outcome}", ask.tool, ask.target));
    }
}

/// `POST /v1/personas/:id/connections` — turn one of an assistant's default
/// connections on or off.
///
/// The write half of a feature whose read half never existed. `setPersonaConnection`
/// has been in the client since the fork, POSTing to a path registered for GET only.
pub(in crate::core::api_server) async fn persona_connection_set(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let connector = body["connector"].as_str().unwrap_or("").trim().to_string();
    let enabled = body["enabled"].as_bool().unwrap_or(false);
    let store = crate::core::persona_connections::PersonaConnections::load(&data_dir);
    if let Err(e) = store.set(&id, &connector, enabled) {
        return Json(json!({ "ok": false, "error": e }));
    }
    // The updated list comes back so the card re-renders from the server's answer
    // rather than from what it hoped the write did.
    let reg = registry(&ctx);
    let rows = reg.manifest(&id).map(|m| default_connections(&data_dir, m)).unwrap_or_default();
    Json(json!({ "ok": true, "default_connections": rows }))
}

/// The skills the assistant driving this session was given. Empty = everything
/// installed, the same convention the prompt catalogue uses.
///
/// One function so the menu, the mute endpoint and the force-run check cannot
/// disagree about what this session may run — three spellings of that question is
/// how a menu ends up offering something the validator then refuses.
/// The skills this session's assistant was given.
///
/// `agent` is the persona the CLIENT is showing, and it is authoritative when the
/// session has no record yet. A session only reaches disk on its first turn, so
/// every freshly-opened conversation used to fall through to the empty vec here —
/// and an empty roster means "everything installed", the right default for a
/// generalist. The result: pick Customer Support, press `/`, and the menu offered
/// all thirty-eight procedures including `aws-posture` and `db-migration`, until
/// you had sent a message. It narrowed only once it no longer mattered.
///
/// Two different facts were being spelled the same way — "this assistant was given
/// no list" and "I have never heard of this session" — and the second one silently
/// took the widest possible meaning. Widening a deliberate list is the exact thing
/// `SkillStore::catalog_text_for` refuses to do for the model; the menu a person
/// reads has to refuse it too.
pub(in crate::core::api_server) fn allowed_skills_for(data_dir: &std::path::Path, session_id: &str, agent: Option<&str>) -> Vec<String> {
    let stored = crate::core::sessions::SessionStore::new(data_dir).get(session_id);
    // The stored session wins: it is what the server will actually run as. The
    // hint only answers for a conversation that does not exist yet.
    let Some(id) = stored.as_ref().map(|s| s.agent.as_str()).or(agent) else {
        return Vec::new();
    };
    crate::core::personas::PersonaRegistry::load(data_dir)
        .manifest(id)
        .map(|m| m.skills.clone())
        .unwrap_or_default()
}

/// `GET /v1/sessions/:id/skills` — the composer's "/" menu.
///
/// The popup has fetched this since the fork against a route that did not exist,
/// so the menu was always empty and a person could not invoke a skill at all —
/// they ran only when the assistant picked one off the catalogue itself.
pub(in crate::core::api_server) async fn session_skills_get(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let store = crate::core::skills::SkillStore::load(&dir);
    let agent = q.get("agent").map(String::as_str).filter(|a| !a.is_empty());
    let allowed = allowed_skills_for(&dir, &id, agent);
    let rows = crate::core::session_skills::SessionSkills::load(&dir).menu(&id, &store, &allowed);
    Json(json!({ "skills": rows }))
}

/// `POST /v1/sessions/:id/skills` — mute or unmute one skill for this session.
///
/// `clear` removes the override rather than setting it true. They mean the same
/// thing today; only the cleared one keeps meaning it if the default ever moves.
pub(in crate::core::api_server) async fn session_skills_set(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let skill = body["skill"].as_str().unwrap_or("").trim().to_string();
    let sessions = crate::core::session_skills::SessionSkills::load(&dir);
    let res = if body["clear"].as_bool().unwrap_or(false) {
        sessions.clear(&id, &skill)
    } else {
        sessions.set(&id, &skill, body["enabled"].as_bool().unwrap_or(true))
    };
    if let Err(e) = res {
        return Json(json!({ "ok": false, "error": e }));
    }
    let store = crate::core::skills::SkillStore::load(&dir);
    let allowed = allowed_skills_for(&dir, &id, None);
    Json(json!({ "ok": true, "skills": sessions.menu(&id, &store, &allowed) }))
}
