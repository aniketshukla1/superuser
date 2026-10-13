use super::super::*;

/// `GET /v1/computers/:persona/frames/:id` — a screen a finished turn kept, as a data URL
/// (the token rides in a header, which an `<img src>` cannot send).
pub(in crate::core::api_server) async fn computer_frame(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path((persona, id)): axum::extract::Path<(String, String)>,
) -> axum::response::Response {
    use base64::Engine as _;
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    // Both checked before either becomes part of a path.
    if !crate::core::agentbrowser::computer::is_plain_id(&persona) || !crate::core::agentbrowser::computer::is_frame_id(&id) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "not a frame" }))).into_response();
    }
    let path = crate::core::agentbrowser::computer::frames_dir(&dir, &persona).join(format!("{id}.jpg"));
    match std::fs::read(&path) {
        Ok(bytes) => Json(json!({
            "ok": true,
            "data_url": format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)),
        }))
        .into_response(),
        // Frames are kept 30 days, and Sign out deletes them: a transcript outlives both.
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({ "ok": false, "error": "That screen is no longer kept." }))).into_response(),
    }
}

/// `WS /ws/computer/:persona` — the live screen, and the user's hands on it.
///
/// Out: `frame` / `stopped` as the browser sends them, and `control` whenever who has the
/// wheel changes. In: mouse, wheel, key and text events — applied only while the user
/// holds control, because a socket being open is not permission.
pub(in crate::core::api_server) async fn computer_socket(
    ws: axum::extract::WebSocketUpgrade,
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(persona): axum::extract::Path<String>,
) -> axum::response::Response {
    let c = match computer_for(&ctx, &persona) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    ws.protocols(["superuser"]).on_upgrade(move |socket| handle_computer_socket(socket, c))
}

pub(in crate::core::api_server) async fn handle_computer_socket(socket: axum::extract::ws::WebSocket, c: Arc<crate::core::agentbrowser::computer::Computer>) {
    use axum::extract::ws::Message as Ws;
    use futures_util::{SinkExt, StreamExt};
    use tokio::sync::broadcast::error::RecvError;

    let (mut tx, mut rx) = socket.split();
    let (mut screen, _watching) = c.watch().await;
    let mut said: Option<crate::core::agentbrowser::control::ControlView> = None;
    // Polled rather than pushed: control changes from three places (the tools, the REST
    // routes, the expiry clock), and a half-second poll of one mutex is simpler than
    // threading a channel through all of them.
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
    loop {
        tokio::select! {
            frame = screen.recv() => match frame {
                Ok(s) => {
                    if tx.send(Ws::Text(json!(s).to_string())).await.is_err() {
                        break;
                    }
                }
                // A slow viewer skips frames rather than falling ever further behind.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            },
            incoming = rx.next() => match incoming {
                Some(Ok(Ws::Text(t))) => {
                    let Ok(m) = serde_json::from_str::<Value>(&t) else { continue };
                    if let Err(e) = c.user_input(&m).await {
                        if tx.send(Ws::Text(json!({ "type": "error", "message": e }).to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                Some(Ok(Ws::Close(_))) | Some(Err(_)) | None => break,
                _ => {}
            },
            _ = tick.tick() => {
                let now = c.control();
                if said.as_ref() != Some(&now) {
                    let mut m = json!(now);
                    m["type"] = json!("control");
                    if tx.send(Ws::Text(m.to_string())).await.is_err() {
                        break;
                    }
                    said = Some(now);
                }
            }
        }
    }
}

/// `GET /v1/ledger` — what has changed in the vault since each source was last read.
///
/// **Peeks, never acknowledges.** `changes_since_read(source, commit)` moves the
/// read marker when `commit` is true, and looking at a page is not the same as
/// having read the items: acknowledging here would mean opening Brain quietly
/// consumed the delta the subconscious was about to evaluate.
pub(in crate::core::api_server) async fn ledger_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let vault = ctx.app.state::<crate::AppState>().memory.vault_path.clone();
    let ledger = crate::core::ledger::Ledger::new(&vault);
    if !ledger.initialized() {
        // Not an error: the ledger creates itself on the first sync, so "no history
        // yet" is the ordinary state of a vault nothing has synced into.
        return Json(json!({ "initialized": false, "snapshots": 0, "sources": [] }));
    }
    let sources: Vec<Value> = ledger
        .read_markers()
        .into_iter()
        .filter_map(|(source, _)| {
            ledger.changes_since_read(&source, false).ok().map(|c| {
                json!({
                    "source": source,
                    "added": c.added.len(),
                    "modified": c.modified.len(),
                    "removed": c.removed.len(),
                    "total": c.total,
                    "summary": c.summary(),
                })
            })
        })
        .collect();
    Json(json!({
        "initialized": true,
        "snapshots": ledger.snapshot_count(),
        "sources": sources,
    }))
}

/// `GET /v1/egress` — what has actually left this machine.
///
/// The rollup comes back alongside the requests because they answer different
/// questions and the page shows both: the list says what happened, the rollup says
/// who has heard from you, which is the one a privacy page exists for.
pub(in crate::core::api_server) async fn egress_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({
        "requests": crate::core::egress::recent(&dir, 300),
        "hosts": crate::core::egress::by_host(&dir),
        // Read from the spine rather than from Settings: what the page should show
        // is what is being *enforced*, and if those two ever disagree the enforced
        // one is the truth.
        "local_only": crate::core::egress::local_only(),
    }))
}

/// `POST /v1/egress/clear` — forget the log. The user's own record of their own
/// traffic; nothing else depends on it.
pub(in crate::core::api_server) async fn egress_clear(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({ "ok": crate::core::egress::clear(&dir) }))
}

/// `POST /v1/egress/local-only` — `{ enabled }`. The hard switch, from the page
/// that shows what it is for.
pub(in crate::core::api_server) async fn egress_set_local_only(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let on = body["enabled"].as_bool().unwrap_or(false);
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.local_only = on;
        // `save` is what refreshes the spine's copy, so this is not just persistence.
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "local_only": crate::core::egress::local_only() }))
}

/// `GET /v1/settings` — the composer's model list comes from here (`models`),
/// plus which surfaces are visible and where state lives. An empty `models`
/// leaves the picker stuck on "Loading models…", so local Ollama models are
/// listed alongside the curated cloud ones the user has a key for.
/// `GET /v1/telegram` — the messaging setup. The bot token is never returned:
/// whether one exists is all the UI needs, and echoing a credential back to a page
/// is how it ends up somewhere it shouldn't be.
pub(in crate::core::api_server) async fn telegram_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let s = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
    Json(json!({
        "connected": crate::integrations::telegram::bot_token().is_some(),
        "chat_id": s.telegram_chat_id,
        "allowed": s.telegram_allowed,
        "inbound": s.telegram_inbound,
        // Inbound is only actually live when a chat is allowlisted — the UI says so
        // rather than showing a toggle that silently does nothing.
        "inbound_live": s.telegram_inbound && !crate::integrations::telegram::allowed_chats(&s).is_empty(),
    }))
}
