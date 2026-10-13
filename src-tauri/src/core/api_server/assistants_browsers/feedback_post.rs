use super::super::*;

/// `GET /v1/subagents` — the worker types a lead can staff.
///
/// Built-ins plus anything the user dropped into `<data_dir>/subagents/`. That
/// folder has always been the extension point, complete with a README written on
/// first run, and nothing in the app has ever mentioned it — so in practice nobody
/// could have known the team was theirs to shape.
pub(in crate::core::api_server) async fn subagents_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let store = crate::core::subagent::SubagentStore::load(&data_dir);
    let types: Vec<Value> = store
        .names()
        .into_iter()
        .filter_map(|n| store.get(&n).cloned())
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "builtin": t.builtin,
                // `null` means "inherits the parent's tools", which is a different
                // thing from an empty allowlist and reads differently in the UI.
                "tools": t.tools,
                "instructions": t.instructions,
            })
        })
        .collect();
    // Which assistants staff a named roster, so the UI can say what each type is for.
    let rosters: Vec<Value> = crate::core::personas::PersonaRegistry::load(&data_dir)
        .list_all()
        .into_iter()
        .filter(|p| !p.subagents.is_empty())
        .map(|p| json!({ "id": p.id, "name": p.name, "subagents": p.subagents }))
        .collect();
    Json(json!({ "types": types, "rosters": rosters }))
}

/// `GET /v1/board?workspace=<path>` — the project's work board.
///
/// What the user thought of one assistant turn.
///
/// The evidence half — model, persona, tools that ran — is read here from the stored
/// session rather than accepted from the caller. A client could send it, and a client
/// could send it wrong; this process already knows.
pub(in crate::core::api_server) async fn feedback_post(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    use crate::core::feedback;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();

    let session = body["session"].as_str().unwrap_or("").to_string();
    if session.is_empty() {
        return Json(json!({ "error": "session is required" }));
    }
    let store = crate::core::sessions::SessionStore::new(&data_dir);
    let loaded = store.get(&session);
    // `message_at` is the key the client should send, and the index is resolved from
    // it here. The transcript the user sees is built from the stored messages but is
    // not one-to-one with them — several UI items come from one message — so an
    // index computed on the client is an index computed against the wrong list. The
    // timestamp is on the message itself and means the same thing at both ends.
    let message = match body["message_at"].as_f64() {
        Some(at) if at > 0.0 => {
            match loaded.as_ref().and_then(|s| s.messages.iter().position(|m| (m.at - at).abs() < 0.001)) {
                Some(i) => i,
                None => return Json(json!({ "error": "no message in that session with that timestamp" })),
            }
        }
        // Index is still accepted, for a caller that has one and no clock.
        _ => match body["message"].as_u64() {
            Some(i) => i as usize,
            None => return Json(json!({ "error": "message_at or message is required" })),
        },
    };
    let verdict = match body["verdict"].as_str() {
        Some("up") => feedback::Verdict::Up,
        Some("down") => feedback::Verdict::Down,
        _ => return Json(json!({ "error": "verdict must be \"up\" or \"down\"" })),
    };
    // A downvote with no reason is `ask_user`'s question-with-no-options, one surface
    // over: it costs a click and yields a number nobody can act on.
    let reason = match verdict {
        feedback::Verdict::Down => match body["reason"].as_str().and_then(feedback::Reason::parse) {
            Some(r) => Some(r),
            None => return Json(json!({ "error": "a downvote needs one of the offered reasons" })),
        },
        feedback::Verdict::Up => None,
    };

    let mut e = feedback::entry(&session, message, verdict);
    e.reason = reason;
    e.note = body["note"].as_str().unwrap_or("").trim().chars().take(2_000).collect();
    if let Some(s) = loaded {
        e.evidence.persona = s.agent.clone();
        e.evidence.model = s.model.clone();
        if let Some(m) = s.messages.get(message) {
            e.message_at = m.at;
            if let Some(wire) = &m.wire {
                let mut seen: Vec<String> = Vec::new();
                for name in wire.tool_names() {
                    if !seen.iter().any(|s| *s == name) {
                        seen.push(name);
                    }
                }
                e.evidence.tools = seen;
            }
        }
    }
    feedback::record(&data_dir, &e);
    Json(json!({ "ok": true }))
}

/// The verdicts already given in one session, so the transcript can show them, plus
/// the reasons a downvote may carry — the front end must not hold its own copy of
/// that list, or the two drift and the store fills with reasons the fold ignores.
pub(in crate::core::api_server) async fn feedback_get(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    use crate::core::feedback;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let reasons: Vec<Value> = feedback::Reason::ALL
        .iter()
        .map(|r| json!({ "id": r.as_str(), "label": r.label() }))
        .collect();
    let mut given: std::collections::BTreeMap<usize, (&str, f64)> = Default::default();
    if let Some(session) = q.get("session").filter(|s| !s.is_empty()) {
        for e in feedback::all(&data_dir).iter().filter(|e| e.session == *session) {
            let v = match e.verdict {
                feedback::Verdict::Up => "up",
                feedback::Verdict::Down => "down",
            };
            given.insert(e.message, (v, e.message_at));
        }
    }
    let given: Vec<Value> = given
        .into_iter()
        .map(|(m, (v, at))| json!({ "message": m, "verdict": v, "message_at": at }))
        .collect();
    Json(json!({ "reasons": reasons, "given": given }))
}

/// The fold, as markdown, for a person deciding what to work on.
pub(in crate::core::api_server) async fn feedback_report_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let entries = crate::core::feedback::all(&data_dir);
    let r = crate::core::feedback::report(&entries);
    Json(json!({ "up": r.up, "down": r.down, "markdown": r.render() }))
}

/// Read-only on purpose. The board's whole value is that a builder cannot grade
/// its own work (`core::board`), and the enforcement rests on the actor being the
/// run's session id, which the platform assigns. A human moving items through a
/// REST call has no such identity — every write would arrive as the same
/// anonymous actor, which is precisely the hole the rule exists to close. So this
/// shows the board; assistants change it.
pub(in crate::core::api_server) async fn board_get(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let Some(workspace) = q.get("workspace").map(|w| w.trim()).filter(|w| !w.is_empty()) else {
        // No project, no board. Not an error: most conversations have no workspace,
        // and the panel asks before it knows whether there is anything to show.
        return Json(json!({ "items": [] }));
    };
    let items = crate::core::board::Board::open(&data_dir, std::path::Path::new(workspace)).items();
    Json(json!({ "items": items }))
}

/// `GET /v1/unrouted` — inbound messages that reached this machine and went
/// nowhere, so the person who owns the bot finds out that someone tried.
///
/// The allowlist dropping an unvouched-for chat is correct. Dropping it *silently*
/// was not: the sender waits forever and nobody here ever learns. See
/// `core::unrouted` for why none of this is ever shown to the agent.
pub(in crate::core::api_server) async fn unrouted_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({ "items": crate::core::unrouted::Unrouted::load(&data_dir).list() }))
}

/// `POST /v1/unrouted/clear` — the user has seen them and dealt with them.
pub(in crate::core::api_server) async fn unrouted_clear(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::core::unrouted::Unrouted::load(&data_dir).clear();
    Json(json!({ "ok": true }))
}
