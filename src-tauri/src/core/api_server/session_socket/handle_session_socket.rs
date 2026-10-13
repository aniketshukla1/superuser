use super::super::*;

mod reattach;
mod user_message;
mod fan_out;
mod run_turn;
mod follow_turn;
mod set_model;

use reattach::*;
use user_message::*;
use fan_out::*;
use run_turn::*;
use follow_turn::*;
use set_model::*;

/// The read half of a session socket.
pub(in crate::core::api_server) type WsStream = futures_util::stream::SplitStream<axum::extract::ws::WebSocket>;

pub(in crate::core::api_server) async fn handle_session_socket(socket: axum::extract::ws::WebSocket, ctx: Arc<ApiContext>, id: String, q: SocketQuery) {
    use axum::extract::ws::Message as Ws;
    use futures_util::StreamExt;

    let (mut tx, mut rx) = socket.split();
    let st = store(&ctx);
    // Resolve the requested persona to one we actually have: the GUI defaults to
    // the "cowork" id. Storing that verbatim makes the sidebar drop the
    // session entirely (it only lists sessions whose agent is a visible persona).
    let reg = registry(&ctx);
    let agent = match (!q.agent.is_empty()).then(|| reg.manifest(&q.agent)).flatten() {
        Some(m) => m.id.clone(),
        None => crate::core::personas::DEFAULT_PERSONA_ID.to_string(),
    };
    // Held in memory only: the GUI opens a socket for every session id it shows,
    // so persisting here would litter the sidebar with blank rows. It's written on
    // the first real turn instead.
    let session = st
        .get(&id)
        .unwrap_or_else(|| crate::core::sessions::Session::with_id(&id, &agent, &q.workspace, ""));

    // The model this session will actually run on — never "".
    //
    // A session with no explicit pick stored `""`, and the GUI read an empty model as
    // "keep the one you already had", so every new chat inherited the previous chat's
    // model and a pick in one chat moved all the others. Resolved here, once, so the
    // two fields built from it cannot describe different models.
    let effective = if session.model.is_empty() {
        use tauri::Manager;
        let s = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
        crate::core::providers::effective_model(&s)
    } else {
        session.model.clone()
    };

    crate::trace!(
        "chat",
        "socket open session={id} agent={agent} model={effective} shared={} running={} stored_model={:?}",
        crate::core::providers::runs_locally(&effective),
        is_running(&id),
        session.model
    );

    // `ready` tells the GUI the socket is live and which session it's bound to.
    // Every frame is {type, data} — the GUI reads its payload from `ev.data`.
    if send(&mut tx, json!({
        "type": "ready",
        "data": {
            "session_id": session.session_id,
            "agent": session.agent,
            // The model this session will actually run on — never "". Resolved once,
            // above, so `model_shared` cannot describe a different model than `model`.
            "model": effective.clone(),
            "mode": session.mode,
            "workspace": session.workspace,
            // Whether every chat shares one endpoint for this model. The GUI uses it
            // to explain a wait instead of leaving it unexplained: a local runner
            // serves a fixed number of sequences at once, so a second chat on the
            // same model is genuinely waiting its turn rather than stuck. Answered
            // here because `providers::runs_locally` is where that rule lives.
            "model_shared": crate::core::providers::runs_locally(&effective),
        }
    })).await.is_err()
    {
        return;
    }

    // Reattaching to a session whose turn is still in flight: replay everything the
    // turn has emitted so far, then follow it live. Without this, coming back to a
    // running chat showed an empty transcript and an empty Progress panel — the work
    // was still happening, it just looked like it had vanished.
    if is_running(&id) && !reattach(&mut tx, &mut rx, &st, &id).await {
        return;
    }

    while let Some(Ok(msg)) = rx.next().await {
        let Ws::Text(txt) = msg else {
            if matches!(msg, Ws::Close(_)) {
                break;
            }
            continue;
        };
        let Ok(frame) = serde_json::from_str::<Value>(&txt) else { continue };
        match frame["type"].as_str().unwrap_or("") {
            "user_message" => on_user_message(&ctx, &mut tx, &mut rx, &st, &id, &session, &frame).await,
            "interrupt" => {
                crate::commands::cancel_session(&id);
                let _ = send(&mut tx, json!({ "type": "interrupted", "data": {} })).await;
            }
            "set_model" | "set_mode" => set_model_or_mode(&mut tx, &st, &id, &frame).await,
            // An answer that arrives when the turn is already over. Not an error and
            // not worth a notice: the user pressed send as the question timed out, or
            // a reconnecting client flushed a queued frame. Dropping it is the point —
            // `ask::answer` is keyed to a waiting question, so there is nothing for a
            // late one to land on.
            "question_response" => {
                let a = reply_text(&frame);
                if !a.is_empty() {
                    crate::core::ask::answer(&id, &a);
                }
            }
            // The user's decision on a tool call the run is blocked on. Late ones
            // are dropped for the same reason a late answer is: the rendezvous is
            // keyed to a waiting call, so there is nothing for one to land on.
            "approval" => {
                let d = frame["decision"].as_str().unwrap_or("").trim();
                if !d.is_empty() {
                    crate::core::ask::answer(&id, d);
                }
            }
            other => {
                // Plans are still answered through the Inbox.
                let _ = send(&mut tx, json!({
                    "type": "notice",
                    "data": { "message": format!("'{other}' isn't handled by this build yet — resolve it in the Inbox.") }
                })).await;
            }
        }
    }
}
