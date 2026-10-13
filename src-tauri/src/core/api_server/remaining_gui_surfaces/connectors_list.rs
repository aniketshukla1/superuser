use super::super::*;

/// The catalog in the GUI's Connector shape. Note `name` (not `id`), and the
/// per-field `required`/`help` its connect form renders.
pub(in crate::core::api_server) async fn connectors_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let settings = crate::core::settings::Settings::load(&dir);
    let unrouted = crate::core::unrouted::Unrouted::load(&dir);

    let items: Vec<Value> = crate::integrations::descriptors::CONNECTORS
        .iter()
        .map(|c| {
            let key = crate::integrations::token_key(c.id);
            let connected = crate::core::secrets::has_integration_token(key);
            let fields: Vec<Value> = c
                .fields
                .iter()
                .map(|f| {
                    json!({
                        "key": f.key,
                        "label": f.label,
                        "secret": f.secret,
                        "required": f.required,
                        "help": f.help,
                        "placeholder": f.placeholder,
                    })
                })
                .collect();
            json!({
                "name": c.id,
                "title": c.title,
                "icon": c.id,
                "logo": c.id,
                "blurb": c.blurb,
                "auth": c.auth,
                "two_way": c.two_way,
                // Chat platforms a session can subscribe to.
                "channels": matches!(c.id, "slack" | "discord" | "telegram" | "whatsapp"),
                // `available` = we have a working client; the rest are catalogued honestly.
                "available": c.implemented,
                "fields": fields,
                "instructions": c.instructions,
                "connected": connected,
                "account": crate::core::secrets::get_account(c.id),
                "enabled": connected,
                "brand_color": c.brand_color,
                "aliases": c.aliases,
                // Who may drive the agent through this connector, and who tried and
                // was refused. Both were declared REQUIRED by the client's
                // `Connector` type and sent by nothing, so the allowlist chips and
                // the parked-message cards never rendered — and the six routes
                // behind them were never called. That is the quiet half of a
                // contract gap: not a 404, but a control that cannot appear.
                "allowed_users": allowed_users(c.id, &settings),
                "unauthorized": unrouted.for_source(c.id).iter().map(parked).collect::<Vec<_>>(),
                // No cloud account, so no managed OAuth broker and nothing that could
                // have come through one. Sent explicitly rather than left out: the
                // client's type requires both, and `undefined` reading as false is
                // an accident that happens to be right, which is not the same as
                // being right.
                "managed": false,
                "managed_profile": false,
                // The assistant tools that reach this connector. An empty list here
                // read as "No tools for this connector yet" on every connector's page,
                // Prometheus included, while assistants were querying it.
                "tools": connector_tools(c.id),
            })
        })
        .collect();
    Json(json!({ "connectors": items }))
}

/// Ids allowed to reach the agent through one connector.
///
/// Only Telegram has an allowlist today — it is the connector that answers anyone
/// who finds the bot, so it is the one that needed a gate. The rest report an empty
/// list, which is honest: they have no per-sender gate rather than a closed one.
pub(in crate::core::api_server) fn allowed_users(connector: &str, settings: &crate::core::settings::Settings) -> Vec<String> {
    match connector {
        "telegram" => crate::integrations::telegram::allowed_chats(settings),
        _ => Vec::new(),
    }
}

/// One parked message in the shape the connector cards read (`ParkedMessage`).
pub(in crate::core::api_server) fn parked(i: &crate::core::unrouted::UnroutedItem) -> Value {
    json!({
        "id": i.id,
        "platform": i.source,
        "chat_id": i.chat_id,
        "chat_name": Value::Null,
        "user_id": i.chat_id,
        "user_name": i.sender,
        "chat_type": "direct",
        "text": i.text,
        "ts": i.ts,
        "reason": i.reason,
    })
}

/// `GET /v1/connectors/:name/tools` — the list `connectors_list` sends with each connector.
pub(in crate::core::api_server) async fn connector_tools_api(axum::extract::Path(name): axum::extract::Path<String>) -> Json<Value> {
    match crate::integrations::descriptors::find(crate::integrations::normalize_id(&name)) {
        Some(c) => Json(json!({ "ok": true, "tools": connector_tools(c.id) })),
        None => Json(json!({ "ok": false, "error": format!("unknown connector: {name}"), "tools": [] })),
    }
}

/// The assistant tools that reach one connector, in the client's `ConnectorTool` shape.
///
/// A list to read, not switches: the tools are generic, so turning one off for a
/// single app would need a gate nothing enforces. `enabled` is therefore always true —
/// any assistant granted the capability can use the tool on this app.
pub(in crate::core::api_server) fn connector_tools(connector: &str) -> Vec<Value> {
    crate::core::agent::tools_using(connector)
        .iter()
        .map(|t| {
            let (label, description, writes) = tool_copy(&t.name)
                .unwrap_or((t.name.as_str(), t.description.as_str(), t.risk != crate::core::agent::RiskClass::Read));
            json!({
                "name": t.name,
                "label": label,
                "kind": if writes { "write" } else { "read" },
                "description": description,
                "enabled": true,
                "requires_approval": writes,
            })
        })
        .collect()
}

/// How the Connectors page words an assistant tool: a label, one line, and whether it
/// changes anything in the connected app.
///
/// Not read off the risk class. `connector_sync` is `External` because it reaches the
/// app, but it only reads it and runs without asking; `send_message` is the one that
/// posts, and the one whose executor stops at `inbox::decide`.
pub(in crate::core::api_server) fn tool_copy(name: &str) -> Option<(&'static str, &'static str, bool)> {
    Some(match name {
        "metrics_find" => ("Find metrics", "Search metric names, with each one's help text and labels.", false),
        "metrics_query" => ("Run a query", "Run a PromQL query for one moment — now, or any time before.", false),
        "metrics_range" => ("Query a time range", "Average, min, max and last for each series a PromQL query returns over a range.", false),
        "metrics_compare" => ("Compare before and after", "Which series moved at a release or config change, and by how much.", false),
        "metrics_alerts" => ("Read alerts", "The alerts firing or pending now. Nothing is silenced or changed.", false),
        "perf_report" => ("Health report", "Your Product Performance signals, compared with the window before.", false),
        "connector_search" => ("Search synced content", "Search what has been synced from this app into memory.", false),
        "connector_sync" => ("Sync now", "Pull the latest from this app into memory.", false),
        "connector_changes" => ("What's new", "What was added, changed or removed since an assistant last looked.", false),
        "send_message" => ("Send a message", "Send a message on your behalf.", true),
        _ => return None,
    })
}

/// `POST /v1/connectors/:name/allow` — let this sender through from now on.
///
/// The other end of `core::unrouted`. Recording a refused message was only half a
/// feature: the Inbox could say "someone tried" and the only way to act on it was
/// to hand-edit a comma-separated field in Settings.
pub(in crate::core::api_server) async fn connector_allow(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let user_id = body["user_id"].as_str().unwrap_or("").trim().to_string();
    if user_id.is_empty() {
        return Json(json!({ "ok": false, "error": "which sender?" }));
    }
    if name != "telegram" {
        return Json(json!({ "ok": false, "error": format!("{name} has no per-sender allowlist in this app") }));
    }
    let mut settings = crate::core::settings::Settings::load(&dir);
    let changed = crate::integrations::telegram::allow(&mut settings, &user_id);
    if changed {
        settings.save(&dir);
        crate::core::audit::log(&dir, "connector", "user", &name, &format!("allowed {user_id}"), true);
    }
    Json(json!({ "ok": true, "changed": changed, "allowed_users": allowed_users(&name, &settings) }))
}
