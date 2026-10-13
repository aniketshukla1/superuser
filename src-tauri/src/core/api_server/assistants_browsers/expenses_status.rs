use super::super::*;

/// `POST /v1/telegram` — set the token (keychain) and the messaging settings.
pub(in crate::core::api_server) async fn telegram_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    if let Some(token) = body["token"].as_str() {
        let t = token.trim();
        if t.is_empty() {
            crate::core::secrets::clear_integration_token("telegram");
        } else if !crate::core::secrets::set_integration_token("telegram", t) {
            return Json(json!({ "ok": false, "error": "couldn't store the token" }));
        }
    }
    {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&body);
        s.save(&state.data_dir);
    }
    let s = state.settings.lock().unwrap().clone();
    Json(json!({
        "ok": true,
        "connected": crate::integrations::telegram::bot_token().is_some(),
        "inbound_live": s.telegram_inbound && !crate::integrations::telegram::allowed_chats(&s).is_empty(),
    }))
}

/// The email bot's state, for Settings (`core::email_bot`): whether it can run, and when it
/// can't, the one sentence why.
pub(in crate::core::api_server) fn email_bot_status(settings: &crate::core::settings::Settings, data_dir: &std::path::Path) -> Value {
    let connected = crate::core::secrets::has_integration_token("email");
    let list = crate::core::email_bot::allowlist(&settings.email_allowed);
    let problem = crate::core::email_bot::drafter_problem(&crate::core::personas::PersonaRegistry::load(data_dir));
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    json!({
        "connected": connected,
        "address": if connected { crate::integrations::email::creds().map(|c| c.address).unwrap_or_default() } else { String::new() },
        "inbound": settings.email_inbound,
        "allowed": settings.email_allowed,
        "allowed_count": list.len(),
        "live": settings.email_inbound && connected && !list.is_empty() && problem.is_none(),
        "problem": problem,
        "drafted_today": crate::integrations::email::drafted_on(&crate::integrations::email::load_state(data_dir), &today),
        "daily_cap": crate::core::email_bot::DAILY_CAP,
    })
}

/// The two settings `POST /v1/email-bot` may change, and nothing else: this route must not
/// become a way to write any setting at all.
pub(in crate::core::api_server) fn email_bot_patch(body: &Value) -> Value {
    let mut patch = serde_json::Map::new();
    if let Some(on) = body.get("email_inbound").filter(|v| v.is_boolean()) {
        patch.insert("email_inbound".into(), on.clone());
    }
    if let Some(list) = body.get("email_allowed").filter(|v| v.is_string()) {
        patch.insert("email_allowed".into(), list.clone());
    }
    Value::Object(patch)
}

pub(in crate::core::api_server) async fn email_bot_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(email_bot_status(&settings, &state.data_dir))
}

pub(in crate::core::api_server) async fn email_bot_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&email_bot_patch(&body));
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = email_bot_status(&settings, &state.data_dir);
    status["ok"] = json!(true);
    Json(status)
}

/// The Expense Manager's state, for Settings (`core::expenses`).
pub(in crate::core::api_server) fn expenses_status(settings: &crate::core::settings::Settings, data_dir: &std::path::Path) -> Value {
    use crate::core::expenses as ex;
    let today = chrono::Local::now().date_naive();
    let all = ex::load(data_dir);
    let this_month = ex::period("this_month", None, None, today)
        .map(|p| {
            all.iter()
                .filter(|e| chrono::NaiveDate::parse_from_str(&e.date, "%Y-%m-%d").is_ok_and(|d| d >= p.from && d <= p.to))
                .count()
        })
        .unwrap_or(0);
    let routine = has_routine(data_dir, ex::ASSISTANT, ex::ROUTINE_TITLE);
    json!({
        "connected": crate::core::secrets::has_integration_token("email"),
        "intake": settings.expense_intake,
        "policy": settings.expense_policy,
        "receipts": all.len(),
        "this_month": this_month,
        "read_today": crate::integrations::email::extracted_on(&crate::integrations::email::load_state(data_dir), &today.format("%Y-%m-%d").to_string()),
        "daily_cap": ex::DAILY_CAP,
        "ledger": ex::ledger_path(data_dir).display().to_string(),
        "routine": routine,
    })
}

/// The two settings `POST /v1/expenses` may change, and nothing else.
pub(in crate::core::api_server) fn expenses_patch(body: &Value) -> Value {
    let mut patch = serde_json::Map::new();
    if let Some(on) = body.get("expense_intake").filter(|v| v.is_boolean()) {
        patch.insert("expense_intake".into(), on.clone());
    }
    if let Some(policy) = body.get("expense_policy").filter(|v| v.is_string()) {
        patch.insert("expense_policy".into(), policy.clone());
    }
    Value::Object(patch)
}

pub(in crate::core::api_server) async fn expenses_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(expenses_status(&settings, &state.data_dir))
}

pub(in crate::core::api_server) async fn expenses_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&expenses_patch(&body));
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = expenses_status(&settings, &state.data_dir);
    status["ok"] = json!(true);
    Json(status)
}

/// Whether a bot's routine exists — the one its Settings panel adds.
pub(in crate::core::api_server) fn has_routine(data_dir: &std::path::Path, agent: &str, title: &str) -> bool {
    crate::core::automation::AutomationStore::load(data_dir).list().iter().any(|t| t.agent == agent && t.title == title)
}

/// Add a bot's routine — once. Checked like any other routine, so it counts toward the
/// switched-on limit and meets the frequency floor.
pub(in crate::core::api_server) fn add_routine(data_dir: &std::path::Path, agent: &str, title: &str, cron: &str, instructions: &str) -> Value {
    use crate::core::automation::{AutomationStore, Schedule, ScheduledTask};
    if has_routine(data_dir, agent, title) {
        return json!({ "ok": true, "created": false });
    }
    let task = ScheduledTask::new(
        title.into(),
        instructions.into(),
        Schedule { kind: "cron".into(), cron: Some(cron.into()), fire_at: None, timezone: "local".into() },
        data_dir.to_string_lossy().into_owned(),
        Some(agent.into()),
        None,
    );
    match AutomationStore::load(data_dir).create_checked(task) {
        Ok(t) => json!({ "ok": true, "created": true, "id": t.id }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The Expense Manager's weekly summary (`core::expenses::ROUTINE_*`).
pub(in crate::core::api_server) fn add_expense_routine(data_dir: &std::path::Path) -> Value {
    use crate::core::expenses as ex;
    add_routine(data_dir, ex::ASSISTANT, ex::ROUTINE_TITLE, ex::ROUTINE_CRON, ex::ROUTINE_INSTRUCTIONS)
}

/// The Chief of Staff's state, for Settings: the priorities, and what feeds the digest.
pub(in crate::core::api_server) fn chief_status(settings: &crate::core::settings::Settings, data_dir: &std::path::Path, memory: &crate::core::memory::MemoryEngine) -> Value {
    use crate::core::chief_of_staff as cos;
    let sources: Vec<Value> = cos::sources(&memory.list(), chrono::Local::now())
        .into_iter()
        .map(|(source, items)| json!({ "source": source, "items": items }))
        .collect();
    json!({
        "priorities": settings.cos_priorities,
        "sources": sources,
        "routine": has_routine(data_dir, cos::ASSISTANT, cos::ROUTINE_TITLE),
    })
}

/// The one setting `POST /v1/chief-of-staff` may change.
pub(in crate::core::api_server) fn chief_patch(body: &Value) -> Value {
    match body.get("cos_priorities").filter(|v| v.is_string()) {
        Some(p) => json!({ "cos_priorities": p }),
        None => json!({}),
    }
}
