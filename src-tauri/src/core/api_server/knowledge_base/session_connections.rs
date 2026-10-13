use super::super::*;

/// The session's own view of what it can touch: every connected connector with its
/// per-session grant resolved, plus what the persona still recommends connecting.
///
/// The GUI's Access panel reads exactly this. `persona` is a hint for a brand-new
/// session with no stored record yet — without it a fresh session would resolve
/// against the DEFAULT persona rather than the one it was born from.
pub(in crate::core::api_server) async fn session_connections(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let session = store(&ctx).get(&id);
    let grants = session.as_ref().map(|s| s.connections.clone()).unwrap_or_default();
    let persona = q
        .get("persona")
        .cloned()
        .or_else(|| session.as_ref().map(|s| s.agent.clone()))
        .unwrap_or_default();

    let connected: Vec<Value> = crate::integrations::descriptors::CONNECTORS
        .iter()
        .filter(|c| {
            c.implemented
                && crate::core::secrets::has_integration_token(crate::integrations::token_key(c.id))
        })
        .map(|c| {
            let g = grants.get(c.id);
            json!({
                "connector": c.id,
                // No grant recorded = live, and as wide as it was before scopes
                // existed. An upgrade must not quietly revoke a send an automation
                // already depends on.
                "enabled": g.map(|g| g.enabled).unwrap_or(true),
                "scope": g
                    .map(|g| g.scope.clone())
                    .unwrap_or_else(|| crate::core::sessions::SCOPE_WRITE.to_string()),
                // Whether a message can actually LEAVE through this source — the
                // authority is the send path itself, not the catalogue's `two_way`
                // flag. They disagree: Discord, WhatsApp and Email are catalogued
                // one-way yet `senders::send` routes all three, so keying the gate
                // on `two_way` would have left three ways out of the machine that
                // the user had no control to close.
                "sendable": crate::core::senders::is_sendable(c.id),
                "detail": crate::core::secrets::get_account(c.id).unwrap_or_default(),
            })
        })
        .collect();

    let recs = registry(&ctx).manifest(&persona).map(|m| m.recommends.clone()).unwrap_or_default();
    let recommended: Vec<Value> = recs
        .iter()
        .filter(|r| r.kind == "connector")
        .filter(|r| {
            !crate::core::secrets::has_integration_token(crate::integrations::token_key(&r.reference))
        })
        .map(|r| json!({ "connector": r.reference, "reason": r.reason, "tier": r.tier, "connected": false }))
        .collect();

    let attention = recommended.len();
    Json(json!({ "connected": connected, "recommended": recommended, "attention": attention }))
}

/// Set one connector's per-session grant. `enabled` and `scope` move independently,
/// so muting a source does not reset the scope the user chose for it; `clear` drops
/// the override so the session inherits the persona default again.
pub(in crate::core::api_server) async fn session_connection_set(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let connector = body["connector"].as_str().unwrap_or("").trim().to_string();
    if connector.is_empty() {
        return Json(json!({ "ok": false, "error": "connector is required" }));
    }
    let ok = store(&ctx).set_connection(
        &id,
        &connector,
        body["enabled"].as_bool(),
        body["scope"].as_str(),
        body["clear"].as_bool().unwrap_or(false),
    );
    if ok {
        Json(json!({ "ok": true }))
    } else {
        Json(json!({ "ok": false, "error": format!("unknown session: {id}") }))
    }
}

/// `POST /v1/personas/:id/enable` — the detail page's Enabled toggle.
///
/// A separate path from `persona_update` because that is the one the client calls,
/// and it answered 404. Returns the refreshed list so the sidebar and the picker
/// re-render from the server rather than from what the toggle hoped happened.
/// `DELETE /v1/personas/:id` — the trash button beside a user-made assistant.
///
/// The client has called this since the fork; the route registered `get`, `post`
/// and `patch` and no `delete`, so the button 405'd and the row stayed.
pub(in crate::core::api_server) async fn persona_delete(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let mut reg = registry(&ctx);
    let name = reg.manifest(&id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
    // Deleting an assistant removes files from disk. Every other consequential act
    // in this app leaves an audit line; this one left none, which made "where did
    // they go?" unanswerable from the record.
    crate::core::audit::log(&data_dir, "persona", "user", &id, &format!("deleted {name}"), true);
    match reg.uninstall(&id) {
        // `list_all`, not `sidebar`. The client replaces its list with whatever comes
        // back, and the settings tab shows every assistant while `sidebar` is the
        // picker's view — enabled AND surfaced only, in a different shape. Returning
        // it here made deleting one disabled assistant appear to delete every
        // disabled assistant, until a tab switch refetched and they all came back.
        Ok(()) => Json(json!({ "ok": true, "personas": reg.list_all() })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

pub(in crate::core::api_server) async fn persona_enable(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mut reg = registry(&ctx);
    let enabled = body["enabled"].as_bool().unwrap_or(false);
    match reg.set_enabled(&id, enabled) {
        // Same shape as the list endpoint — see `persona_delete`. Disabling an
        // assistant here would otherwise have hidden every other disabled one.
        Ok(()) => Json(json!({ "ok": true, "personas": reg.list_all() })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}
