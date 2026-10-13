use super::super::*;

pub(in crate::core::api_server) async fn handle_context_command(
    ctx: &Arc<ApiContext>,
    st: &crate::core::sessions::SessionStore,
    session: &crate::core::sessions::Session,
    text: &str,
) -> Option<String> {
    use crate::core::context;
    use tauri::Manager;

    let cmd = text.split_whitespace().next().unwrap_or("");
    if !matches!(cmd, "/context" | "/compact" | "/undo") {
        return None;
    }

    if cmd == "/undo" {
        use tauri::Manager;
        let state = ctx.app.state::<crate::AppState>();
        let (data_dir, vault) = (state.data_dir.clone(), state.memory.vault_path.clone());
        let Some(snap) = crate::core::evolution::latest_snapshot(&data_dir) else {
            return Some("Nothing to undo — I haven't changed any of my own files since you last put something back.".into());
        };
        let scope = crate::core::evolution::Scope::for_review(&data_dir, &vault);
        let n = crate::core::evolution::restore(&snap, &scope);
        // Recorded as rejected so it is not proposed again — see `journal_undo`.
        crate::core::wiki::Wiki::new(&vault).mark_undone(&snap.id);
        crate::core::evolution::forget_snapshot(&data_dir, &snap.id);
        return Some(format!(
            "Put back {n} file{} to how {} were before my last self-learning pass.",
            if n == 1 { "" } else { "s" },
            if n == 1 { "it" } else { "they" },
        ));
    }

    let settings = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
    let model = if session.model.is_empty() { settings.ollama_model.clone() } else { session.model.clone() };
    let local = crate::core::providers::resolve(&model).is_none();
    let budget = context::Budget::resolve(&settings, local);

    let as_json = |h: Vec<(String, String)>| -> Vec<Value> {
        h.into_iter().map(|(role, content)| json!({ "role": role, "content": content })).collect()
    };

    if cmd == "/context" {
        let s = context::stats(&as_json(session.history()), budget);
        let compacted = if session.compact_from > 0 {
            format!("\n- **Compacted:** the first {} messages are represented by a summary", session.compact_from)
        } else {
            String::new()
        };
        return Some(format!(
            "**Context for this session**\n\n\
             - **Carried in:** {} messages ({} from you, {} from me), ~{} characters\n\
             - **Estimated tokens:** ~{} of {} ({}% of the budget)\n\
             - **Limits:** {} turns carried in, {} tool steps per turn\n\
             - **Model:** {} ({}){}\n\n\
             Run `/compact` to fold the older turns into a summary and free up room.",
            s.messages, s.user, s.assistant, s.chars, s.est_tokens, s.max_tokens, s.pct,
            s.max_turns, budget.max_steps, model,
            if local { "local — the budget is capped by your Ollama context window" } else { "cloud" },
            compacted,
        ));
    }

    // /compact
    let keep = compact_keep_turns(budget.max_turns);
    let Some(split) = compact_split(&session.messages, session.compact_from, keep) else {
        return Some(format!(
            "Nothing to compact yet — this session is still within its last {keep} turns. Run `/context` to see where it stands."
        ));
    };

    // Everything from the previous compaction point up to the split, plus whatever
    // the last compaction already summarized, so no earlier material is lost.
    let mut material: Vec<Value> = Vec::new();
    if !session.compact_summary.is_empty() {
        material.push(json!({ "role": "assistant", "content": format!("Summary so far:\n{}", session.compact_summary) }));
    }
    material.extend(
        session.messages[session.compact_from..split]
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| json!({ "role": m.role, "content": m.content })),
    );

    let before = context::estimate_messages(&as_json(session.history()));
    let summary = context::summarize_dropped(&settings, &material).await;
    st.set_compaction(&session.session_id, split, &summary);

    let after = st
        .get(&session.session_id)
        .map(|s| context::estimate_messages(&as_json(s.history())))
        .unwrap_or(before);
    Some(format!(
        "**Compacted.** Folded {} earlier messages into a summary and kept the last {keep} turns in full.\n\n\
         Context carried into the next turn: ~{} tokens, down from ~{}.\n\n\
         Your transcript above is unchanged — this only affects what I'm told going forward.",
        split - session.compact_from,
        after,
        before,
    ))
}

/// `WS /ws/events` — the app-wide event stream (sessions changing, automation runs
/// finishing, inbox arrivals). The GUI opens it at startup and retries hard if it
/// fails, so it must exist even while we have little to push. It holds the socket
/// open and answers pings; per-turn detail flows on the session socket.
pub(in crate::core::api_server) async fn events_socket(ws: axum::extract::WebSocketUpgrade, AxState(ctx): AxState<Arc<ApiContext>>) -> axum::response::Response {
    ws.protocols(["superuser"]).on_upgrade(move |socket| async move {
        use axum::extract::ws::Message as Ws;
        use futures_util::StreamExt;
        let (mut tx, mut rx) = socket.split();
        let _ = send(&mut tx, json!({ "type": "ready", "data": { "service": "superuser" } })).await;
        let _ = &ctx;
        // Stay open until the client goes away; reply to pings so idle sockets live.
        while let Some(Ok(msg)) = rx.next().await {
            match msg {
                Ws::Close(_) => break,
                Ws::Ping(p) => {
                    use futures_util::SinkExt;
                    if tx.send(Ws::Pong(p)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        }
    })
}

/// `WS /ws/session/:id` — the live transcript. The client sends `user_message`
/// (and control frames); we stream `turn_start` → tool steps → `assistant_message`
/// → `turn_done`, and persist the turn so a reload replays it.
pub(in crate::core::api_server) async fn session_socket(
    ws: axum::extract::WebSocketUpgrade,
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<SocketQuery>,
) -> axum::response::Response {
    ws.protocols(["superuser"]).on_upgrade(move |socket| handle_session_socket(socket, ctx, id, q))
}

/// One run event, in the shape the GUI's session socket reads — or `None` for a kind
/// the GUI has no use for.
///
/// Extracted from the forwarder because this mapping is a contract with two ends and a
/// trapdoor in the middle: **an event kind not listed here never reaches the GUI at
/// all.** That is not hypothetical. `question_requested` was handled in App.tsx, typed
/// in types.ts and rendered by InboxItemCard, and no question ever appeared — nothing
/// emitted the kind, and had anything done so, this `None` would have eaten it. A
/// silent drop is the failure mode here, so the mapping is testable and tested.
/// What the user sent back, as the one string the rendezvous carries.
///
/// The card sends one answer per question (`answers`), null for anything skipped.
/// It travels as a string because `core::ask` is deliberately ignorant of what was
/// asked — it joins a waiting run to a reply and nothing else — and the tool that
/// asked is the one that knows how to read it back (`Ask::read_answer`).
///
/// `answer` is still accepted: that is what an Inbox reply and any older client send.
pub(in crate::core::api_server) fn reply_text(frame: &Value) -> String {
    if let Some(a) = frame["answer"].as_str() {
        return a.trim().to_string();
    }
    match &frame["answers"] {
        // A question card answers with a list, in question order.
        Value::Array(a) if !a.is_empty() => json!({ "answers": a }).to_string(),
        // A form card answers with an object keyed by field name. Same frame, same
        // rendezvous: only the shape of the answer differs, and the tool that parked
        // the receiver knows which one it asked for.
        Value::Object(o) if !o.is_empty() => json!({ "answers": o }).to_string(),
        _ => String::new(),
    }
}
