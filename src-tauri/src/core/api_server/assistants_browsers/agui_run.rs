use super::super::*;

/// `POST /v1/agui/:persona` — this assistant, as an AG-UI agent.
///
/// AG-UI is what every CopilotKit surface speaks to an agent over: a
/// POST carrying the conversation, and a stream of typed events back. Serving it means a
/// Superuser assistant can be the agent behind somebody else's chat window — with its own
/// prompt, tools, boundaries and trail, and none of this app's UI involved.
///
/// The tools run HERE. AG-UI lets a surface offer tools for itself to execute; ours stay
/// where the gateway, the boundaries and the approvals are, so a call is reported as it
/// happens and never handed out to be run somewhere this app cannot see.
pub(in crate::core::api_server) async fn agui_run(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(persona): axum::extract::Path<String>,
    Json(input): Json<Value>,
) -> axum::response::Response {
    use crate::core::agui;
    use tauri::Manager;

    let ids = agui::Ids::from_input(&input);
    let reg = registry(&ctx);
    let persona = match reg.manifest(&persona) {
        Some(m) if reg.is_enabled(&m.id) => m.id.clone(),
        // Said as a run error rather than a 404: the caller is a chat window, and a
        // stream that never starts leaves it waiting with nothing to show.
        _ => {
            let body = agui::sse(&agui::run_error(&ids, &format!("There's no assistant called “{persona}” here.")));
            return sse_response(futures_util::stream::once(async move { Ok::<_, std::convert::Infallible>(body) }));
        }
    };
    let Some(prompt) = agui::last_user_message(&input) else {
        let body = agui::sse(&agui::run_error(&ids, "That run carried no message to answer."));
        return sse_response(futures_util::stream::once(async move { Ok::<_, std::convert::Infallible>(body) }));
    };
    let history: Vec<crate::commands::HistMsg> =
        agui::history(&input).into_iter().map(|(role, text)| crate::commands::HistMsg::text(&role, &text)).collect();
    // The conversation keeps its own session, so a thread in somebody else's window has a
    // memory here the same way a conversation in ours does. The id is theirs, filed under
    // a prefix of ours and reduced to characters a path can hold.
    let session = (!ids.thread.is_empty()).then(|| {
        let safe: String = ids.thread.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(48).collect();
        format!("agui-{safe}")
    });

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let app = ctx.app.clone();
    let running = tauri::async_runtime::spawn(async move {
        let state = app.state::<crate::AppState>();
        crate::commands::run_agent_streaming(&app, &state, persona, prompt, session, None, Some(history), tx).await
    });

    // The events, as strings, on a channel the response streams from. A channel rather
    // than a generator because this app carries no generator crate, and the shape is the
    // same: one task writes the run's events, the response drains them.
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tauri::async_runtime::spawn(async move {
        let send = |line: String| {
            let _ = out_tx.send(line);
        };
        send(agui::sse(&agui::run_started(&ids)));
        let mut text_open = false;
        while let Some(ours) = rx.recv().await {
            for event in agui::events_for(&ours, &ids, &mut text_open) {
                send(agui::sse(&event));
            }
        }
        match running.await {
            Ok(Ok(out)) => {
                // A model that did not stream still answered; without this the surface
                // gets RUN_STARTED, RUN_FINISHED and an empty reply.
                if text_open {
                    if let Some(end) = agui::text_end(&ids, text_open) {
                        send(agui::sse(&end));
                    }
                } else {
                    for event in agui::whole_message(&ids, &out.answer) {
                        send(agui::sse(&event));
                    }
                }
                send(agui::sse(&agui::run_finished(&ids)));
            }
            Ok(Err(e)) => {
                if let Some(end) = agui::text_end(&ids, text_open) {
                    send(agui::sse(&end));
                }
                send(agui::sse(&agui::run_error(&ids, &e)));
            }
            Err(e) => send(agui::sse(&agui::run_error(&ids, &format!("that run stopped: {e}")))),
        }
    });

    sse_response(futures_util::stream::unfold(out_rx, |mut rx| async move {
        rx.recv().await.map(|line| (Ok::<_, std::convert::Infallible>(line), rx))
    }))
}

/// The same headers every SSE answer needs, in one place.
pub(in crate::core::api_server) fn sse_response<S>(stream: S) -> axum::response::Response
where
    S: futures_util::Stream<Item = Result<String, std::convert::Infallible>> + Send + 'static,
{
    use axum::response::IntoResponse;
    (
        [
            (axum::http::header::CONTENT_TYPE, "text/event-stream"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
            (axum::http::header::CONNECTION, "keep-alive"),
        ],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

/// `POST /v1/route` — `{message, session_id?}`: which assistant a message that names
/// nobody should go to, and why.
///
/// Answers even when it could not decide: the reply carries the reason, and `chose` says
/// whether it was a decision or the default. A router that quietly returns the default as
/// though it had picked is the failure this shape exists to prevent.
pub(in crate::core::api_server) async fn route_message(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let reg = registry(&ctx);
    let default_id = reg.default_id();
    let message = body["message"].as_str().unwrap_or("");
    let choice = crate::core::router::pick(&settings, &reg, message, &default_id).await;
    let session = body["session_id"].as_str().unwrap_or("");
    crate::core::router::record(&state.data_dir, session, &choice);
    Json(json!({
        "ok": true,
        "persona": choice.persona,
        "why": choice.why.label(),
        "chose": choice.why.chose(),
        "confidence": choice.confidence,
    }))
}

/// `POST /v1/sessions/:id/grants` — `{"grant": "browser@<host>[:<port>]"}`: the "Always allow
/// on <site>" a person gives on the first card, given ahead of time for one conversation.
///
/// Only a scratch instance takes one — an app started with `SUPERUSER_DATA_DIR`, which is
/// what the harness requires of every task that changes configuration. It exists so the
/// harness can stand in for the click a person would make; on anyone's own app that click
/// stays theirs, and this answers 403. Browser grants only: a command or a send approved
/// ahead of time is a different conversation to have.
pub(in crate::core::api_server) async fn session_grant(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    use tauri::Manager;
    if std::env::var("SUPERUSER_DATA_DIR").map(|d| d.trim().is_empty()).unwrap_or(true) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "Only a scratch instance (SUPERUSER_DATA_DIR) accepts approvals ahead of time." })),
        )
            .into_response();
    }
    let grant = body["grant"].as_str().unwrap_or("").trim();
    let host = grant.strip_prefix("browser@").unwrap_or("");
    let plain = |s: &str| !s.is_empty() && s.len() <= 200 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    // A site is a host and, when it names one, a port — the same shape a person's "Always
    // allow" grants (`agent_exec::site_of`).
    let site = |s: &str| {
        let (h, port) = match s.rsplit_once(':') {
            Some((h, p)) if !h.is_empty() && !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h, Some(p)),
            _ => (s, None),
        };
        let h = h.strip_prefix('[').and_then(|v| v.strip_suffix(']')).unwrap_or(h);
        port.is_none_or(|p| p.parse::<u16>().is_ok()) && (plain(h) || (port.is_some() && h.chars().all(|c| c.is_ascii_hexdigit() || c == ':')))
    };
    if !plain(&id) || !site(host) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "expected {\"grant\": \"browser@<host>[:<port>]\"}" }))).into_response();
    }
    crate::core::cmdgate::grants::allow_tool(&id, grant);
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::core::audit::log(&dir, "approve", &id, host, &format!("acting on {host} allowed ahead of time, on a scratch instance"), true);
    Json(json!({ "ok": true })).into_response()
}
