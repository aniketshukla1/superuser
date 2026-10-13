use super::super::*;

/// One parked ask, as the Inbox lists it.
pub(in crate::core::api_server) fn inbox_item(a: &crate::core::inbox::Ask) -> Value {
    let resolved = a.status != "pending";
    json!({
        "id": a.id,
        "session_id": a.session_id,
        // A parked `ask_user` is a question, not a permission request. Rendered
        // as an approval it would offer Approve/Deny for "which quarter?" —
        // buttons that cannot answer it.
        "kind": if a.tool == "ask_user" { "question" } else { "approval" },
        "title": a.summary,
        "body": if a.tool == "ask_user" { String::new() } else { a.args.to_string() },
        // The Inbox card renders quick replies as plain strings, so the
        // first question's option labels are what it gets. The full
        // multi-question card belongs to the live path; a parked ask is
        // answered in prose, and `Ask::read_answer` takes prose.
        "options": inbox_options(&a.args),
        "allow_text": a.args["allow_text"].as_bool().unwrap_or(true),
        "multi": a.args["multi"].as_bool().unwrap_or(false),
        "state": if resolved { "resolved" } else { "pending" },
        "resolution": if resolved { Some(a.status.clone()) } else { None },
        "inbox": "main",
        "created_at": iso_secs(a.created_at),
        "resolved_at": a.resolved_at.map(iso_secs),
        "tool": a.tool,
        "target": a.target,
        // What the Inbox card reads to dress a parked approval like a live one — the humane
        // title, and the text of what would be sent or written. The card has looked here
        // since the fork and nothing sent it, so every parked approval fell back to its raw
        // arguments: a drafted email arrived as a JSON string.
        "data": { "tool": a.tool, "arguments": a.args },
    })
}

pub(in crate::core::api_server) async fn inbox_list(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<InboxQuery>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let items: Vec<Value> = crate::core::inbox::Inbox::load(&dir)
        .list()
        .into_iter()
        .filter(|a| inbox_matches(a, &q))
        .map(|a| inbox_item(&a))
        .collect();
    Json(json!({ "items": items }))
}

pub(in crate::core::api_server) fn mcp_registry(ctx: &ApiContext) -> crate::mcp::registry::McpRegistry {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::mcp::registry::McpRegistry::new(&dir)
}

/// `GET /v1/mcp` — every server the MCP tab should show: the connectors we ship,
/// then the ones the user configured.
///
/// This used to answer with `McpConnector` (`{id, name, url, connected}`), which
/// carries none of the fields the GUI's contract declares — reading `status` off one
/// of those rows threw during render and took the whole tab down. Rows are built from
/// the shared row-builders now, and a test pins that every declared field is present.
pub(in crate::core::api_server) async fn mcp_list_api(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    let mut servers = crate::mcp::registry::builtin_rows();
    servers.extend(mcp_registry(&ctx).list().iter().map(|e| e.to_row()));
    Json(json!({ "servers": servers }))
}

/// `POST /v1/mcp` — add (or correct) a server.
pub(in crate::core::api_server) async fn mcp_add_api(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    let name = body["name"].as_str().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Json(json!({ "ok": false, "error": "a server needs a name" }));
    }
    if crate::mcp::connectors::CONNECTORS.iter().any(|c| c.name == name) {
        return Json(json!({ "ok": false, "error": format!("{name} is a built-in connector — pick another name") }));
    }
    let config = body.get("config").cloned().unwrap_or_else(|| json!({}));
    if config.get("url").is_none() && config.get("command").is_none() {
        return Json(json!({ "ok": false, "error": "a server config needs a `url` (http) or a `command` (stdio)" }));
    }
    // The floor for an address somebody pasted (Phase 6). Everything this app holds is on
    // the other end of it, so plain http, credentials in the URL, this machine's own
    // network and names nobody can check are refused here rather than at the first call.
    if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
        if let Some(problem) = crate::core::contentgov::mcp_endpoint_problem(url) {
            return Json(json!({ "ok": false, "error": format!("That server wasn't added: {problem}") }));
        }
    }
    if mcp_registry(&ctx).upsert(&name, config) {
        Json(json!({ "ok": true }))
    } else {
        Json(json!({ "ok": false, "error": "couldn't write the server registry" }))
    }
}

/// `PATCH /v1/mcp/:name` — flip a switch or replace a config.
pub(in crate::core::api_server) async fn mcp_patch_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    if crate::mcp::connectors::CONNECTORS.iter().any(|c| c.name == name) {
        return Json(json!({ "ok": false, "error": format!("{name} ships with the app and can't be edited — sign out to disconnect it") }));
    }
    // A replaced config meets the same floor a new one does: an edit is how a checked
    // address becomes an unchecked one.
    if let Some(url) = body.get("config").and_then(|c| c.get("url")).and_then(|v| v.as_str()) {
        if let Some(problem) = crate::core::contentgov::mcp_endpoint_problem(url) {
            return Json(json!({ "ok": false, "error": format!("That change wasn't saved: {problem}") }));
        }
    }
    let ok = mcp_registry(&ctx).patch(
        &name,
        body["enabled"].as_bool(),
        body["requires_approval"].as_bool(),
        body.get("config").filter(|c| c.is_object()).cloned(),
    );
    if ok {
        Json(json!({ "ok": true }))
    } else {
        Json(json!({ "ok": false, "error": format!("unknown server: {name}") }))
    }
}

/// `DELETE /v1/mcp/:name`
pub(in crate::core::api_server) async fn mcp_delete_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    if crate::mcp::connectors::CONNECTORS.iter().any(|c| c.name == name) {
        return Json(json!({ "ok": false, "error": format!("{name} ships with the app and can't be removed") }));
    }
    if mcp_registry(&ctx).remove(&name) {
        Json(json!({ "ok": true }))
    } else {
        Json(json!({ "ok": false, "error": format!("unknown server: {name}") }))
    }
}

/// Resolve a server name to the (credential id, url) the OAuth flows need. Only
/// HTTP servers can sign in, so a stdio server is refused here by construction.
pub(in crate::core::api_server) fn mcp_endpoint(ctx: &ApiContext, name: &str) -> Result<(String, String), String> {
    if let Some(c) = crate::mcp::connectors::CONNECTORS.iter().find(|c| c.name == name) {
        return Ok((c.id.to_string(), c.url.to_string()));
    }
    let Some(e) = mcp_registry(ctx).get(name) else { return Err(format!("unknown server: {name}")) };
    match e.url() {
        Some(u) => Ok((name.to_string(), u.to_string())),
        // Only reached by the OAuth flows. A stdio server is launched locally and
        // has no endpoint to sign in to, so there is nothing here to browse to —
        // it can still be listed, inspected and called like any other server.
        None => Err(format!("{name} is a {} server — it runs locally, so there is no sign-in to do", e.transport())),
    }
}

/// `GET /v1/mcp/:name/tools` — ask the server what it can do.
pub(in crate::core::api_server) async fn mcp_tools_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    let client = crate::core::egress::client("mcp");
    // Through the shared link, so a stdio server answers here exactly as an HTTP
    // one does — this endpoint used to be able to reach only the HTTP half.
    let opened = if let Some(c) = crate::mcp::connectors::CONNECTORS.iter().find(|c| c.name == name) {
        crate::mcp::McpLink::open_builtin(c.id, c.url, &client).await
    } else if let Some(e) = mcp_registry(&ctx).get(&name) {
        crate::mcp::McpLink::open_entry(&e, &client).await
    } else {
        return Json(json!({ "ok": false, "error": format!("unknown server: {name}"), "tools": [] }));
    };
    match opened {
        Err(e) => Json(json!({ "ok": false, "error": e, "tools": [] })),
        Ok(mut sess) => match sess.list_tools(&client).await {
            Ok(tools) => {
                let items: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t["name"].as_str().unwrap_or_default(),
                            "description": t["description"].as_str().unwrap_or_default(),
                        })
                    })
                    .collect();
                Json(json!({ "ok": true, "tools": items }))
            }
            Err(e) => Json(json!({ "ok": false, "error": e, "tools": [] })),
        },
    }
}
