use super::super::*;

/// `POST /v1/mcp/reload` — the registry is read from disk on every request, so
/// there is no cache to drop; this exists so the GUI's "reload now" is honest.
pub(in crate::core::api_server) async fn mcp_reload_api(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    Json(json!({ "ok": true, "servers": mcp_registry(&ctx).list().len() }))
}

/// `POST /v1/mcp/:name/connect` — run the browser OAuth flow for this server.
pub(in crate::core::api_server) async fn mcp_connect_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    let (id, url) = match mcp_endpoint(&ctx, &name) {
        Ok(v) => v,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    match crate::mcp::oauth::connect(ctx.app.clone(), &id, &url, None).await {
        Ok(()) => Json(json!({ "ok": true, "started": true })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/mcp/:name/signout` — drop this server's stored credentials.
pub(in crate::core::api_server) async fn mcp_signout_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    let id = match mcp_endpoint(&ctx, &name) {
        Ok((id, _)) => id,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    Json(json!({ "ok": crate::mcp::oauth::forget(&id) }))
}

/// `GET /v1/audit` — the trail, newest first.
///
/// Filters: `session_id`, `persona`, `event` (a name or a family, `action`),
/// `nobody_watching`, `refused`, `since` (epoch seconds), `tool`, `connector`, `limit`.
/// The Actions tab has sent `session_id`, `connector` and `tool` since it was built,
/// and this route ignored all three — so its filter box filtered nothing.
pub(in crate::core::api_server) async fn audit_list(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let flag = |k: &str| q.get(k).is_some_and(|v| v == "1" || v == "true");
    let text = |k: &str| q.get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let limit = q.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(200).clamp(1, 2000);
    let tool = text("tool");
    let connector = text("connector").map(|c| c.to_lowercase());
    let filter = crate::core::audit::Filter {
        // Asked for everything that matches the filters the core knows; the two it
        // doesn't are applied below, and the page is cut after them rather than before.
        limit: usize::MAX,
        session: text("session_id"),
        persona: text("persona"),
        event: text("event"),
        nobody_watching: flag("nobody_watching"),
        refused: flag("refused"),
        decisions: flag("decisions"),
        since: q.get("since").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
    };
    let events: Vec<Value> = tokio::task::spawn_blocking(move || {
        crate::core::audit::query(&dir, &filter)
            .into_iter()
            .filter(|e| {
                tool.as_deref().is_none_or(|t| e.subject.as_ref().is_some_and(|s| s.tool == t) || e.kind == t || e.event_name() == t)
            })
            .filter(|e| connector.as_deref().is_none_or(|c| e.target.to_lowercase().contains(c)))
            .take(limit)
            .enumerate()
            .map(|(i, e)| audit_row_json(i, e))
            .collect()
    })
    .await
    .unwrap_or_default();
    Json(json!({ "events": events }))
}

/// One trail row as the GUI reads it: the first version's fields, which the Actions
/// tab was built against, and version 2's beside them.
pub(in crate::core::api_server) fn audit_row_json(i: usize, e: crate::core::audit::Entry) -> Value {
    let status = if e.refused() {
        "refused"
    } else if e.ok {
        "ok"
    } else {
        "error"
    };
    let tool = e.subject.as_ref().map(|s| s.tool.clone()).unwrap_or_else(|| e.kind.clone());
    let session = if e.session.is_empty() { e.actor.clone() } else { e.session.clone() };
    let agent = if e.persona.is_empty() { e.actor.clone() } else { e.persona.clone() };
    let unwatched = e.initiator.as_ref().is_some_and(|i| i.unwatched());
    json!({
        "id": i,
        "timestamp": iso_secs(e.at),
        "session_id": session,
        "agent": agent,
        "workspace": "",
        "connector": if e.kind == "sync" { e.target.clone() } else { String::new() },
        "tool": tool,
        "stage": "done",
        "status": status,
        "approval": "",
        "args": { "target": e.target.clone() },
        "result_preview": e.detail.clone(),
        "at": e.at,
        "event": e.event_name(),
        "actor": e.actor,
        "target": e.target,
        "detail": e.detail,
        "ok": e.ok,
        "persona": e.persona,
        "initiator": e.initiator,
        "unwatched": unwatched,
        "subject": e.subject,
        "decision": e.decision,
    })
}

/// `GET /v1/audit/verify` — walk the trail's chain and say whether it still adds up.
pub(in crate::core::api_server) async fn audit_verify(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let found = tokio::task::spawn_blocking(move || crate::core::audit::verify(&dir)).await.unwrap_or_default();
    Json(json!(found))
}

/// `GET /v1/boundaries` — what is in force, plus everything the page needs to edit it:
/// the presets with their costs, the fields a rule can read, and any rule that no
/// longer checks out.
pub(in crate::core::api_server) async fn boundaries_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let loaded = crate::core::boundaries::load(&dir);
    Json(json!({
        "boundaries": loaded.boundaries,
        "problem": loaded.problem,
        "problems": crate::core::boundaries::check(&loaded.boundaries),
        "presets": crate::core::boundaries::PRESETS,
        "fields": crate::core::boundaries::FIELDS
            .iter()
            .map(|(field, means)| json!({ "field": field, "means": means }))
            .collect::<Vec<_>>(),
    }))
}

/// `POST /v1/boundaries` — save the whole set. Nothing invalid is saved: the answer
/// lists every problem, by list and position, for the page to show beside its rule.
pub(in crate::core::api_server) async fn boundaries_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let candidate: crate::core::boundaries::Boundaries = match serde_json::from_value(body) {
        Ok(b) => b,
        Err(e) => return Json(json!({ "ok": false, "error": format!("those boundaries aren't in a shape this app reads: {e}") })),
    };
    match crate::core::boundaries::save(&dir, &candidate) {
        Ok(()) => Json(json!({ "ok": true, "boundaries": candidate })),
        Err(problems) => Json(json!({ "ok": false, "problems": problems })),
    }
}

/// `POST /v1/boundaries/check` — is this one rule usable? For checking as it is typed.
pub(in crate::core::api_server) async fn boundaries_check(Json(body): Json<Value>) -> Json<Value> {
    let when = body["when"].as_str().unwrap_or("");
    let problem = crate::core::boundaries::check_rule(when);
    Json(json!({ "ok": problem.is_none(), "problem": problem }))
}

/// `POST /v1/boundaries/test` — what a candidate set would have decided about the
/// actions already on the trail. Replays, never saves.
pub(in crate::core::api_server) async fn boundaries_test(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let candidate: crate::core::boundaries::Boundaries = match serde_json::from_value(body["boundaries"].clone()) {
        Ok(b) => b,
        Err(e) => return Json(json!({ "ok": false, "error": format!("those boundaries aren't in a shape this app reads: {e}") })),
    };
    let problems = crate::core::boundaries::check(&candidate);
    let limit = body["limit"].as_u64().unwrap_or(1000).clamp(1, 20_000) as usize;
    let report = tokio::task::spawn_blocking(move || {
        let rows = crate::core::audit::query(&dir, &crate::core::audit::Filter { decisions: true, limit, ..Default::default() });
        crate::core::boundaries::replay(&candidate, &rows)
    })
    .await
    .unwrap_or_default();
    Json(json!({ "ok": true, "report": report, "problems": problems }))
}
