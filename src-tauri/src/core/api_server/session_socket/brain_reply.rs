use super::super::*;

pub(in crate::core::api_server) type WsSink = futures_util::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>;

/// Send one JSON frame to the client.
pub(in crate::core::api_server) async fn send(tx: &mut WsSink, v: Value) -> Result<(), axum::Error> {
    use futures_util::SinkExt;
    tx.send(axum::extract::ws::Message::Text(v.to_string())).await
}

#[derive(serde::Deserialize)]
pub(in crate::core::api_server) struct SocketQuery {
    #[serde(default)]
    pub(in crate::core::api_server) workspace: String,
    #[serde(default)]
    pub(in crate::core::api_server) agent: String,
}

/// Turns kept verbatim by `/compact`. Everything older is folded into one summary.
/// A quarter of the carry-in allowance, so compacting reliably buys back room
/// rather than shaving a turn off the top.
pub(in crate::core::api_server) fn compact_keep_turns(max_turns: usize) -> usize {
    (max_turns / 4).max(2)
}

/// Index of the message that starts the `keep`-th most recent turn — the point
/// before which everything gets summarized. `None` when the session has fewer
/// turns than that and there is nothing worth compacting.
pub(in crate::core::api_server) fn compact_split(msgs: &[crate::core::sessions::Message], already: usize, keep: usize) -> Option<usize> {
    let starts: Vec<usize> = msgs
        .iter()
        .enumerate()
        .skip(already)
        .filter(|(_, m)| m.role == "user")
        .map(|(i, _)| i)
        .collect();
    // Needs at least one whole turn to summarize on top of the ones being kept.
    if starts.len() <= keep {
        return None;
    }
    Some(starts[starts.len() - keep])
}

/// `/context` and `/compact`. Returns `Some(reply)` when `text` was a command —
/// the caller then answers without running a turn. Anything else returns `None`
/// and is treated as an ordinary message, so a user writing about "/compact" in
/// a sentence is unaffected.
/// Answer one `@brain …` question, or describe the brain when the mention arrives
/// bare.
///
/// Nothing borrowed from `AppState` is held across the await: the engine is rebuilt
/// from the vault path (it is a path and nothing else) and the settings are cloned,
/// so the state lock is released before a model call that can take a minute on a
/// local machine. Holding it would freeze every other command for the duration.
///
/// A failure comes back as prose rather than an error frame, because from the
/// user's side this is a message they sent — "Ollama isn't reachable" belongs in the
/// conversation where they asked, not in a toast that outlives the question.
pub(in crate::core::api_server) async fn brain_reply(ctx: &Arc<ApiContext>, tx: &mut WsSink, question: &str, model: Option<&str>) -> String {
    use crate::core::brain;
    use tauri::Manager;

    let (settings, vault, data_dir) = {
        let state = ctx.app.state::<crate::AppState>();
        let settings = state.settings.lock().unwrap().clone();
        (settings, state.memory.vault_path.clone(), state.data_dir.clone())
    };
    let engine = crate::core::memory::MemoryEngine::new(vault.clone());

    if question.is_empty() {
        return brain::help(&brain::census(&engine, &vault, &data_dir));
    }

    // Progress, as the two slow halves happen.
    //
    // Both are slow enough to need saying — searching can rebuild an embedding
    // index over the whole vault, and the model call runs for minutes on a local
    // one — and neither said anything, so the panel sat on "Waiting for agent…"
    // with a rising timer and no clue whether it was reading files, waiting on a
    // provider, or wedged. These are the same `tool_proposed`/`tool_finished`
    // frames a normal turn's steps ride on, so they render in the panel that
    // already exists rather than needing a shape of their own.
    let step = |name: &str, summary: String| {
        json!({ "type": "tool_proposed", "data": { "name": name, "arguments": {}, "summary": summary } })
    };
    let done = |name: &str, preview: String| {
        json!({ "type": "tool_finished", "data": { "name": name, "status": "ok", "result_preview": preview } })
    };

    let p = brain::plan(&settings, model);
    crate::trace!("brain", "ask model={} q={}", p.model, crate::core::trace::preview(question));

    let _ = send(tx, step("search_brain", "Searching memories, timeline, knowledge and profile".into())).await;
    let r = brain::recall(&engine, &p.settings, &vault, &data_dir, question, p.budget).await;
    let _ = send(tx, done("search_brain", format!("Found {}", r.found()))).await;

    // The model is named while it is being waited on, not after: which provider is
    // taking the minutes is the single most useful thing to know at that moment,
    // and it is also the answer to "is it even reaching a model".
    let _ = send(tx, step("ask_model", format!("Asking {}", p.model))).await;
    match brain::answer(&p, &r, question).await {
        Ok(a) => {
            crate::trace!("brain", "answered by {} ({} chars)", a.provider, a.text.len());
            let _ = send(tx, done("ask_model", format!("Answered by {}", a.provider))).await;
            a.text
        }
        Err(e) => {
            crate::trace!("brain", "failed on {}: {e}", p.model);
            // Reported as a failed step too, not only as prose: a step left hanging
            // spins under the message that says it stopped.
            let _ = send(
                tx,
                json!({ "type": "tool_finished", "data": { "name": "ask_model", "status": "error", "result_preview": e } }),
            )
            .await;
            format!("I couldn't search my memory just now — {e}")
        }
    }
}
