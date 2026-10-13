use super::*;

/// What the server needs to run an assistant turn.
pub struct ApiContext {
    pub app: tauri::AppHandle,
    pub token: String,
}

pub struct RunningApi {
    pub port: u16,
    pub token: String,
    /// True when the preferred port was taken and the OS picked one instead. The caller
    /// says so in the log — serving somewhere other than where you were told to look is
    /// exactly the thing that should not be silent.
    pub fell_back: bool,
    pub(super) shutdown: Option<tokio::sync::watch::Sender<bool>>,
    /// Resolves when the server stops serving, and says whether it was asked to. Taken by
    /// whoever supervises it; see `start_local_api`.
    pub ended: Option<tokio::sync::oneshot::Receiver<Ending>>,
}

/// Why the server stopped serving.
#[derive(Debug, Clone, PartialEq)]
pub enum Ending {
    /// `stop()` was called. Nothing to restart.
    Stopped,
    /// It stopped without being asked: an accept loop that errored, or a crash.
    Failed(String),
}

impl RunningApi {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(true);
        }
    }
}

pub(super) fn unauthorized() -> (StatusCode, Json<Value>) {
    (StatusCode::UNAUTHORIZED, Json(json!({"error": {"message": "invalid or missing bearer token", "type": "invalid_request_error"}})))
}

/// Accepts either `Authorization: Bearer <token>` (OpenAI-compatible clients) or
/// `X-Superuser-Token: <token>` (the GUI's own header).
pub(super) fn authed(headers: &HeaderMap, expected: &str) -> bool {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        // RFC 7235 makes the scheme case-insensitive, and OpenAI-compatible
        // clients in the wild send `bearer`. Rejecting those was an interop bug,
        // not a security measure.
        .and_then(|v| {
            let (scheme, rest) = v.split_once(' ')?;
            scheme.eq_ignore_ascii_case("bearer").then(|| rest.trim())
        })
        .map(|t| secret_eq(t, expected))
        .unwrap_or(false);
    let gui = headers
        .get("x-superuser-token")
        .and_then(|v| v.to_str().ok())
        .map(|t| secret_eq(t, expected))
        .unwrap_or(false);
    bearer || gui
}

/// Compare in time that doesn't depend on how much of the token matched.
///
/// `==` on strings returns at the first differing byte, which leaks the length of
/// the correct prefix to anything that can time the response. Locally that is a
/// weak channel, but the whole point of the token is that "on this machine" is
/// not the same as "trusted", and the fix is three lines.
pub(super) fn secret_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    // The length itself is not a secret, and comparing unequal lengths bytewise
    // would need a decision about how to pad.
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `GET /v1/models` — the personas, presented as models.
pub(super) async fn models(AxState(ctx): AxState<Arc<ApiContext>>, headers: HeaderMap) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !authed(&headers, &ctx.token) {
        return Err(unauthorized());
    }
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let data: Vec<Value> = crate::core::personas::PersonaRegistry::load(&state.data_dir)
        .list_all()
        .into_iter()
        .map(|p| json!({ "id": p.id, "object": "model", "owned_by": "superuser", "name": p.name }))
        .collect();
    Ok(Json(json!({ "object": "list", "data": data })))
}

/// `POST /v1/chat/completions` — runs the assistant loop for the requested persona
/// and returns its answer in the OpenAI response shape. Streaming isn't supported;
/// a `stream: true` request is refused rather than silently answered non-streamed.
pub(super) async fn chat_completions(
    AxState(ctx): AxState<Arc<ApiContext>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !authed(&headers, &ctx.token) {
        return Err(unauthorized());
    }
    if body["stream"].as_bool().unwrap_or(false) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": {"message": "streaming isn't supported by this endpoint yet", "type": "invalid_request_error"}})),
        ));
    }
    let persona = body["model"].as_str().unwrap_or(crate::core::personas::DEFAULT_PERSONA_ID).to_string();
    let msgs = body["messages"].as_array().cloned().unwrap_or_default();
    let prompt = msgs
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();
    if prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": {"message": "no user message", "type": "invalid_request_error"}})),
        ));
    }
    // Earlier turns become history so multi-turn clients work.
    let history: Vec<crate::commands::HistMsg> = msgs
        .iter()
        .take(msgs.len().saturating_sub(1))
        .filter_map(|m| {
            let role = m["role"].as_str()?;
            let content = m["content"].as_str()?;
            (role == "user" || role == "assistant").then(|| crate::commands::HistMsg::text(role, content))
        })
        .collect();

    // `model` above is the *persona*, which is this surface's convention. Which LLM
    // runs behind it is a different question, and one a caller has had no way to ask
    // — so the harness eval could not run a task against a matrix of models through
    // the only headless entry point the app has. Both live under `superuser`, beside
    // the non-standard fields the response already returns.
    let llm = body["superuser"]["model"].as_str().filter(|m| !m.is_empty()).map(String::from);
    // A caller-chosen session id fixes the working folder — `session_dir` is
    // deterministic for a session with no record — which is how a fixture can be put
    // in place before the turn and read back after it.
    let session = body["superuser"]["session_id"].as_str().filter(|m| !m.is_empty()).map(String::from);

    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let out = crate::commands::run_agent(&ctx.app, &state, persona.clone(), prompt, session, llm, Some(history))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": {"message": e, "type": "server_error"}}))))?;

    let measure = out.measure.clone();
    Ok(Json(json!({
        "id": format!("chatcmpl-{}", out.working_dir.chars().rev().take(12).collect::<String>()),
        "object": "chat.completion",
        "created": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "model": persona,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": out.answer },
            "finish_reason": "stop"
        }],
        // Standard, and absent until now: a client had no way to know what a turn
        // cost. Summed across every step of the turn, not just the last call.
        "usage": out.usage.as_ref().map(|u| json!({
            "prompt_tokens": u.input,
            "completion_tokens": u.output,
            "total_tokens": u.input + u.output,
        })),
        // Non-standard but useful: what the assistant actually produced, what it did
        // to produce it, and what that took. The trace is the tool calls in the order
        // they were made — the thing an eval has to assert on, and which no response
        // has ever carried.
        "superuser": {
            "artifacts": out.artifacts,
            "working_dir": out.working_dir,
            "todos": out.todos,
            "model": out.provider,
            "trace": out.wire.as_ref().map(|w| json!(crate::core::transcript::tool_trace(w))),
            "measure": measure.map(|m| json!({
                "steps": m.steps,
                "seconds": m.seconds,
                "cost_usd": m.cost_usd,
                "cache_hit_rate": m.cache_hit_rate,
            })),
        }
    })))
}
