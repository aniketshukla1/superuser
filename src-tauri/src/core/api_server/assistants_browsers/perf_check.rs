use super::super::*;

pub(in crate::core::api_server) async fn chief_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(chief_status(&settings, &state.data_dir, &state.memory))
}

pub(in crate::core::api_server) async fn chief_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&chief_patch(&body));
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = chief_status(&settings, &state.data_dir, &state.memory);
    status["ok"] = json!(true);
    Json(status)
}

pub(in crate::core::api_server) async fn chief_routine(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    use crate::core::chief_of_staff as cos;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(add_routine(&dir, cos::ASSISTANT, cos::ROUTINE_TITLE, cos::ROUTINE_CRON, cos::ROUTINE_INSTRUCTIONS))
}

/// Bug Reproduction's state, for Settings: the staging address, and what its browser needs to
/// reach it — the browser switched on, loopback allowed for a dev server on this Mac, and
/// local-only off for anything else.
pub(in crate::core::api_server) fn repro_status(settings: &crate::core::settings::Settings) -> Value {
    let host = crate::core::boundaries::host_of(&crate::core::browser::normalize_url(&settings.repro_staging));
    json!({
        "staging": settings.repro_staging,
        "browser": settings.browser_enabled,
        "on_this_mac": !host.is_empty() && crate::core::agentbrowser::proxy::is_loopback_name(&host),
        "loopback": settings.browser_allow_loopback,
        "local_only": settings.local_only,
    })
}

/// The one setting `POST /v1/bug-repro` may change, checked: `Err` says what's wrong with it.
pub(in crate::core::api_server) fn repro_patch(body: &Value) -> Result<Value, String> {
    match body.get("repro_staging").and_then(|v| v.as_str()) {
        Some(url) => match crate::core::repro::staging_problem(url) {
            Some(problem) => Err(problem),
            None => Ok(json!({ "repro_staging": url })),
        },
        None => Ok(json!({})),
    }
}

pub(in crate::core::api_server) async fn repro_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(repro_status(&settings))
}

pub(in crate::core::api_server) async fn repro_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let patch = match repro_patch(&body) {
        Ok(p) => p,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&patch);
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = repro_status(&settings);
    status["ok"] = json!(true);
    Json(status)
}

/// Product Performance's state, for Settings: whether Prometheus is connected and as what, the
/// signals and what's wrong with any of them, and whether the weekly report runs.
pub(in crate::core::api_server) fn perf_status(settings: &crate::core::settings::Settings, data_dir: &std::path::Path) -> Value {
    use crate::core::perf;
    use crate::integrations::prometheus::ID;
    let (list, problems) = perf::signals(&settings.perf_signals);
    json!({
        "connected": crate::core::secrets::has_integration_token(ID),
        "account": crate::core::secrets::get_account(ID),
        "signals": settings.perf_signals,
        "count": list.len(),
        "problems": problems,
        "routine": has_routine(data_dir, perf::ASSISTANT, perf::ROUTINE_TITLE),
    })
}

/// The one setting `POST /v1/perf` may change, checked: a line that isn't a signal is said,
/// not saved.
pub(in crate::core::api_server) fn perf_patch(body: &Value) -> Result<Value, String> {
    let Some(text) = body.get("perf_signals").and_then(|v| v.as_str()) else {
        return Ok(json!({}));
    };
    let (_, problems) = crate::core::perf::signals(text);
    match problems.into_iter().find(|p| p.starts_with("line")) {
        Some(p) => Err(format!("L{}.", &p[1..])),
        None => Ok(json!({ "perf_signals": text })),
    }
}

pub(in crate::core::api_server) async fn perf_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(perf_status(&settings, &state.data_dir))
}

pub(in crate::core::api_server) async fn perf_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let patch = match perf_patch(&body) {
        Ok(p) => p,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&patch);
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = perf_status(&settings, &state.data_dir);
    status["ok"] = json!(true);
    Json(status)
}

/// `POST /v1/perf/check` — each signal run once, now, so a wrong query is found in Settings
/// rather than in Monday's report.
pub(in crate::core::api_server) async fn perf_check(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use crate::integrations::prometheus::{Answer, Conn};
    use tauri::Manager;
    let signals = ctx.app.state::<crate::AppState>().settings.lock().unwrap().perf_signals.clone();
    let conn = match Conn::load() {
        Ok(c) => c,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let now = chrono::Utc::now().timestamp() as f64;
    let mut results = Vec::new();
    for sig in crate::core::perf::signals(&signals).0 {
        results.push(match conn.instant(&sig.query, now).await {
            Ok((Answer::Series(_, total), _)) => json!({ "name": sig.name, "ok": true, "series": total }),
            Ok(_) => json!({ "name": sig.name, "ok": true, "series": 1 }),
            Err(e) => json!({ "name": sig.name, "ok": false, "error": e }),
        });
    }
    Json(json!({ "ok": true, "results": results }))
}

pub(in crate::core::api_server) async fn perf_routine(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use crate::core::perf;
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(add_routine(&dir, perf::ASSISTANT, perf::ROUTINE_TITLE, perf::ROUTINE_CRON, perf::ROUTINE_INSTRUCTIONS))
}

pub(in crate::core::api_server) async fn expenses_routine(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(add_expense_routine(&dir))
}
