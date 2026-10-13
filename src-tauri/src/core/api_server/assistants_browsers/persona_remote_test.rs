use super::super::*;

/// The browser for a persona that exists. Creating one only registers it; nothing is
/// launched until something is opened in it.
pub(in crate::core::api_server) fn computer_for(ctx: &ApiContext, persona: &str) -> Result<Arc<crate::core::agentbrowser::computer::Computer>, (StatusCode, Json<Value>)> {
    use tauri::Manager;
    if registry(ctx).manifest(persona).is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "ok": false, "error": format!("There's no assistant called “{persona}”.") }))));
    }
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::core::agentbrowser::computer::get(&dir, persona).map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": e }))))
}

/// `GET /v1/computers` — every assistant's browser: whether it was given one, what it is
/// on, who has the wheel, and whether it has sign-ins that Sign out would delete.
pub(in crate::core::api_server) async fn computers_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let (enabled, allow_loopback, headed) = {
        let s = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
        (s.browser_enabled, s.browser_allow_loopback, s.browser_headed)
    };
    let mut computers = Vec::new();
    for p in registry(&ctx).list_all() {
        let Ok(c) = computer_for(&ctx, &p.id) else { continue };
        let mut row = json!(c.status().await);
        row["name"] = json!(p.name);
        row["icon"] = json!(p.icon);
        row["granted"] = json!(p.tools.iter().any(|t| t == "browser"));
        computers.push(row);
    }
    Json(json!({
        "enabled": enabled,
        "allow_loopback": allow_loopback,
        "headed": headed,
        // Said up front, so the page can tell someone before they switch it on.
        "chrome_found": crate::core::agentbrowser::chrome::find().is_some(),
        "local_only": crate::core::egress::local_only(),
        "computers": computers,
    }))
}

/// `GET /v1/computers/:persona`
pub(in crate::core::api_server) async fn computer_status(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(persona): axum::extract::Path<String>) -> axum::response::Response {
    match computer_for(&ctx, &persona) {
        Ok(c) => Json(json!(c.status().await)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/computers/:persona/stop` — close it and keep its sign-ins.
pub(in crate::core::api_server) async fn computer_stop(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(persona): axum::extract::Path<String>) -> axum::response::Response {
    match computer_for(&ctx, &persona) {
        Ok(c) => Json(json!({ "ok": true, "was_running": c.stop("Stopped by the user.").await })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/computers/:persona/reset` — Sign out of everything: the profile and every
/// kept screen are deleted. The page confirms first; this does not ask again.
pub(in crate::core::api_server) async fn computer_reset(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(persona): axum::extract::Path<String>) -> axum::response::Response {
    match computer_for(&ctx, &persona) {
        Ok(c) => match c.reset().await {
            Ok(()) => Json(json!({ "ok": true })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": e }))).into_response(),
        },
        Err(e) => e.into_response(),
    }
}

/// `POST /v1/computers/:persona/control` — `{"action": "take" | "release"}`.
pub(in crate::core::api_server) async fn computer_control(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(persona): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let c = match computer_for(&ctx, &persona) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match body["action"].as_str().unwrap_or("") {
        "take" => c.take_control(),
        "release" => c.release_control(),
        // "Not now" to a request for help: the assistant is told so, rather than waiting it out.
        "decline" => {
            c.decline_help();
        }
        other => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": format!("unknown action “{other}” — take, release or decline") })))
                .into_response()
        }
    }
    Json(json!({ "ok": true, "control": c.control() })).into_response()
}

/// `POST /v1/computers/:persona/open` — `{"url"}`: open a site for the user to sign in
/// on, with the wheel already theirs.
pub(in crate::core::api_server) async fn computer_open(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(persona): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    use tauri::Manager;
    if !ctx.app.state::<crate::AppState>().settings.lock().unwrap().browser_enabled {
        return (StatusCode::CONFLICT, Json(json!({ "ok": false, "error": "Turn the assistants' browser on first." }))).into_response();
    }
    let c = match computer_for(&ctx, &persona) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let url = crate::core::browser::normalize_url(body["url"].as_str().unwrap_or(""));
    match c.open_for_user(&url).await {
        Ok(page) => Json(json!({ "ok": true, "url": page.url, "title": page.title, "control": c.control() })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": e }))).into_response(),
    }
}

/// `POST /v1/computers/:persona/secret` — `{"value"}` types the value the assistant asked
/// for into its field; `{"decline": true}` says no. The value is never stored, logged or
/// returned: the reply is how many characters arrived.
pub(in crate::core::api_server) async fn computer_secret(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(persona): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let c = match computer_for(&ctx, &persona) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    if body["decline"].as_bool().unwrap_or(false) {
        return Json(json!({ "ok": true, "declined": c.decline_secret() })).into_response();
    }
    match c.supply_secret(body["value"].as_str().unwrap_or("")).await {
        Ok(chars) => Json(json!({ "ok": true, "chars": chars })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": e }))).into_response(),
    }
}

/// `POST /v1/personas/:id/remote/test` — `{endpoint?, token?}`: does this address answer as
/// an AG-UI agent?
///
/// Tests what is typed, before it is saved, so a wrong address is found at the form rather
/// than on the first real turn. The saved token is only ever sent to the saved address's
/// host: testing a new address with an old credential would hand it to whoever answers.
pub(in crate::core::api_server) async fn persona_remote_test(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let reg = registry(&ctx);
    if reg.manifest(&id).is_none() {
        return Json(json!({ "ok": false, "error": format!("unknown assistant: {id}") }));
    }
    let saved = reg.remote(&id);
    let endpoint = body["endpoint"].as_str().map(str::trim).filter(|e| !e.is_empty()).map(str::to_string).or(saved.clone());
    let Some(endpoint) = endpoint else {
        return Json(json!({ "ok": false, "error": "Give the agent's address to test." }));
    };
    let token = body["token"].as_str().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).or_else(|| {
        let same_host = saved.as_deref().is_some_and(|s| crate::core::boundaries::host_of(s) == crate::core::boundaries::host_of(&endpoint));
        same_host.then(|| crate::core::secrets::get_agent_token(&id)).flatten()
    });
    let http = crate::core::egress::client_no_follow("remote-agent");
    match crate::core::aguiclient::test(&http, &endpoint, token.as_deref()).await {
        Ok(message) => Json(json!({ "ok": true, "message": message })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}
