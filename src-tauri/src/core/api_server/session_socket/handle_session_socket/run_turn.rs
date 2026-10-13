use super::*;

/// Runs a turn in the background, follows it on this socket, and reports how it ended.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_turn(ctx: &Arc<ApiContext>, tx: &mut WsSink, rx: &mut WsStream, st: &crate::core::sessions::SessionStore, id: &String, current: &crate::core::sessions::Session, text: String, model: Option<String>, history: Vec<crate::commands::HistMsg>) {
    let _ = send(tx, json!({ "type": "turn_start", "data": {} })).await;

    // Stream the run's steps as they happen.
    let (etx, mut erx) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let app = ctx.app.clone();
    let persona = current.agent.clone();
    // Another assistant answering this turn (an `@` mention): its reply is marked as
    // theirs, on disk and on the socket, so the transcript and the next turn can tell.
    let answered_by = st.get(id).filter(|s| s.agent != persona).map(|_| persona.clone());
    let sid = id.clone();
    // The turn owns its own persistence. It used to be saved by the
    // socket handler AFTER awaiting this task — so switching chats
    // closed the socket, dropped that future, and threw the answer away
    // even though the agent had kept computing. Now the work is saved by
    // whoever is doing it, and the socket is only an observer.
    let dir_for_run = {
        use tauri::Manager;
        ctx.app.state::<crate::AppState>().data_dir.clone()
    };
    let sid_for_run = id.clone();
    mark_running(&sid_for_run, true);
    // Frames go to the feed, not straight to this socket, so they survive
    // the socket closing and can be replayed on reattach — and, via the
    // run's own folder, long after the turn is over.
    let run_dir = crate::core::agent_exec::session_dir_path(&dir_for_run, &sid_for_run);
    let feed = feed_open(&sid_for_run, run_dir.clone());
    // The turn's own boundaries don't travel through the feed (they're
    // socket-level, not agent-level), so the journal records them here.
    // Without the question and the answer, a record of the tool calls in
    // between says what happened but not what it was for.
    crate::core::runlog::append(
        &run_dir,
        &json!({ "type": "turn_start", "data": { "text": text, "model": model.clone().unwrap_or_default() } }),
    );
    {
        let feed = feed.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(ev) = erx.recv().await {
                let Some(out) = gui_frame(&ev) else { continue };
                feed_push(&feed, out);
            }
        });
    }
    // "Run now" arrives here as an ordinary message in the run's session. Its
    // opening message gets the frame a scheduled firing gets; the transcript and
    // the journal above keep the person's words. See `automation::frame_for_turn`.
    let text = crate::core::automation::frame_for_turn(&dir_for_run, &sid, text);
    let answered_by_run = answered_by.clone();
    let run = tauri::async_runtime::spawn(async move {
        use tauri::Manager;
        let state = app.state::<crate::AppState>();
        let out = crate::commands::run_agent_streaming(&app, &state, persona, text, Some(sid), model, Some(history), etx).await;
        let store = crate::core::sessions::SessionStore::new(&dir_for_run);
        match &out {
            Ok(o) => {
                let mut m = crate::core::sessions::Message::new("assistant", o.answer.clone())
                    .with_usage(o.usage.clone());
                // H2: the turn's own tool traffic, so the next turn can see
                // what this one already read and ran instead of doing it
                // again. Separate from `items`, which is what the transcript
                // renderer draws.
                m.wire = o.wire.clone();
                m.items = o.todos.iter().map(|t| json!({ "kind": "todo", "content": t.content, "status": t.status })).collect();
                // Charts ride out on the assistant message the same way todos
                // and fan-out threads do. Without this the transcript loses the
                // chart on reload while the answer above it still says "as
                // shown" — the store keeps the final text and nothing else.
                m.items.extend(o.charts.iter().cloned());
                m.persona = answered_by_run;
                store.append(&sid_for_run, m);
            }
            Err(e) => {
                // Record the failure too: a turn that vanished without
                // explanation is worse than one that says it failed.
                let mut m = crate::core::sessions::Message::new("assistant", format!("That turn failed: {e}"));
                m.persona = answered_by_run;
                store.append(&sid_for_run, m);
            }
        }
        mark_running(&sid_for_run, false);
        // Whatever this turn was waiting to be told, it is past being told
        // it. Left behind, the slot would hand the NEXT turn's ask_user an
        // answer to a question it never asked.
        crate::core::ask::abandon(&sid_for_run);
        // A steer that arrived just after the last boundary has missed
        // its turn. Dropping it here is deliberate: carrying it into the
        // *next* turn would apply an instruction to work it was never
        // about. It is already in the transcript, so nothing is hidden.
        clear_steers(&sid_for_run);
        // Drop the replay buffer; the transcript is persisted now, so a
        // later reattach reads messages rather than replaying frames.
        feed_close(&sid_for_run);
        out
    });

    // Best-effort reporting only; the task above already saved the result.
    let result = follow_turn(tx, rx, st, id, feed, run).await;
    match result {
        Ok(out) => {
            // Journalled before it is sent: the socket may already be
            // gone (switching chats mid-turn), and the whole point of
            // the record is that it doesn't depend on anyone watching.
            crate::core::runlog::append(&run_dir, &json!({ "type": "assistant_message", "data": { "text": out.answer, "usage": out.usage, "measure": out.measure, "persona": answered_by } }));
            crate::core::runlog::append(&run_dir, &json!({ "type": "turn_done", "data": { "artifacts": out.artifacts } }));
            // `measure` rides with `usage` rather than in its own frame: what a turn
            // cost and how long it took are one fact about one turn, and a
            // second frame could arrive without the first (or not at all).
            let _ = send(tx, json!({ "type": "assistant_message", "data": { "text": out.answer, "usage": out.usage, "measure": out.measure, "persona": answered_by } })).await;
            let _ = send(tx, json!({
                "type": "turn_done",
                "data": { "artifacts": out.artifacts, "todos": out.todos }
            })).await;
        }
        Err(e) => {
            crate::core::runlog::append(&run_dir, &json!({ "type": "error", "data": { "message": e } }));
            let _ = send(tx, json!({ "type": "error", "data": { "message": e } })).await;
        }
    }
}
