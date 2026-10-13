use super::super::*;

/// `POST /v1/settings/computer` — the assistants' browser: on or off, whether it may open
/// this machine's own dev servers, and whether it shows a window.
///
/// Switching it off stops every browser now rather than at next use — a switch that
/// leaves Chrome running behind it is not off. Sign-ins are kept.
pub(in crate::core::api_server) async fn settings_set_computer(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let (enabled, allow_loopback, headed, headed_changed) = {
        let mut s = state.settings.lock().unwrap();
        let before = s.browser_headed;
        let mut patch = serde_json::Map::new();
        for key in ["browser_enabled", "browser_allow_loopback", "browser_headed"] {
            if let Some(v) = body.get(key).and_then(|v| v.as_bool()) {
                patch.insert(key.into(), json!(v));
            }
        }
        s.apply_patch(&Value::Object(patch));
        s.save(&state.data_dir);
        (s.browser_enabled, s.browser_allow_loopback, s.browser_headed, before != s.browser_headed)
    };
    // A window can't be added to a running browser: the next start shows the change.
    if !enabled || headed_changed {
        let reason = if enabled { "Restarting to show the change to its window." } else { "The assistants' browser was switched off." };
        for c in crate::core::agentbrowser::computer::all() {
            c.stop(reason).await;
        }
    }
    Json(json!({ "ok": true, "browser_enabled": enabled, "browser_allow_loopback": allow_loopback, "browser_headed": headed }))
}

/// `POST /v1/settings/scratch-base` — where NEW conversations get their folder.
///
/// The card promises "existing conversations keep their current folder", and that
/// promise is why this route did not exist for so long: while a session's folder
/// was derived from `data_dir + id`, honouring the setting would have moved every
/// past conversation and orphaned its files. Sessions now pin their folder on
/// first use, so the setting can mean what the card says it means.
///
/// Validated by actually writing: a path can be non-empty, absolute, and still
/// unusable — a typo'd volume, a directory owned by root, a full disk. Reporting
/// "saved" and failing at the first deliverable is the outcome worth avoiding.
pub(in crate::core::api_server) async fn settings_set_scratch_base(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let raw = body["path"].as_str().unwrap_or("").trim().to_string();
    let state = ctx.app.state::<crate::AppState>();
    let data_dir = state.data_dir.clone();

    // Empty clears it, back to the default.
    if raw.is_empty() {
        let mut s = state.settings.lock().unwrap();
        s.scratch_base = String::new();
        s.save(&data_dir);
        return Json(json!({ "ok": true, "scratch_base": data_dir.join("sessions").to_string_lossy() }));
    }

    let expanded = match raw.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().map(|h| h.join(rest)).unwrap_or_else(|| std::path::PathBuf::from(&raw)),
        None => std::path::PathBuf::from(&raw),
    };
    if !expanded.is_absolute() {
        return Json(json!({ "ok": false, "error": "Give a full path, starting from the top of the disk." }));
    }
    if let Err(e) = std::fs::create_dir_all(&expanded) {
        return Json(json!({ "ok": false, "error": format!("Couldn't use that location: {e}") }));
    }
    let probe = expanded.join(".superuser-write-probe");
    if let Err(e) = std::fs::write(&probe, b"") {
        return Json(json!({ "ok": false, "error": format!("That folder isn't writable: {e}") }));
    }
    let _ = std::fs::remove_file(&probe);

    let chosen = expanded.to_string_lossy().to_string();
    {
        let mut s = state.settings.lock().unwrap();
        s.scratch_base = chosen.clone();
        s.save(&data_dir);
    }
    Json(json!({ "ok": true, "scratch_base": chosen }))
}

/// Which session surfaces the sidebar offers, in the shape the client declares.
///
/// A function so it can be tested. What went out before was a different concept's
/// keys entirely — `automations`, `inbox`, `connectors`, `personas`, `skills` —
/// against a client whose `SurfaceVisibility` is `{cowork, chat, code}`. The object
/// was truthy, so `App.tsx` adopted it, and every field it actually reads came back
/// undefined. Including `cowork`, which the sidebar treats as always visible.
pub(in crate::core::api_server) fn surfaces_payload(s: &crate::core::settings::Settings) -> Value {
    // `cowork` is not a setting: it is the surface that is always there, and the
    // one the sidebar falls back to when a gated surface is hidden.
    json!({ "cowork": true, "chat": s.surface_chat, "code": s.surface_code })
}

/// `POST /v1/settings/onboarded` — Settings' "replay the setup guide".
///
/// The button has been there since the fork; the route has not, and `GET
/// /v1/settings` reported `onboarded` as the constant `true`. So first-run setup
/// could never appear and the button could never bring it back. `App.tsx` has
/// been reading the flag the whole time — this is the half that was missing.
pub(in crate::core::api_server) async fn settings_set_onboarded(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let want = body["onboarded"].as_bool().unwrap_or(false);
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.onboarded = want;
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "onboarded": want }))
}

/// `POST /v1/settings/sessions-peek` — how many conversations a sidebar group
/// shows before "Show more".
///
/// Clamped rather than rejected: the control is a number input, so a stray
/// keystroke produces 500 as easily as 5, and refusing it would leave the card
/// showing an error for something with an obvious sensible reading.
pub(in crate::core::api_server) async fn settings_set_sessions_peek(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let want = body["sessions_peek"].as_u64().unwrap_or(5).clamp(1, 50) as u32;
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.sessions_peek = want;
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "sessions_peek": want }))
}

/// `POST /v1/settings/context-bar` — show the context meter, or the token total.
pub(in crate::core::api_server) async fn settings_set_context_bar(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let want = body["context_bar"].as_bool().unwrap_or(false);
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.context_bar = want;
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "context_bar": want }))
}

/// `POST /v1/settings/standing-instructions` — what every assistant is told, every
/// turn, in the user's words. Empty clears them.
pub(in crate::core::api_server) async fn settings_set_standing(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let text = body["text"].as_str().unwrap_or("");
    let saved = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&json!({ "standing_instructions": text }));
        s.save(&state.data_dir);
        s.standing_instructions.clone()
    };
    // Their length, not their words: the trail says the instructions changed, and
    // the instructions themselves are one click away in the app.
    crate::core::audit::log(
        &state.data_dir,
        "settings",
        "user",
        "standing instructions",
        &if saved.is_empty() { "cleared".to_string() } else { format!("{} characters", saved.chars().count()) },
        true,
    );
    Json(json!({
        "ok": true,
        "standing_instructions": saved,
        "max": crate::core::settings::STANDING_INSTRUCTIONS_MAX,
    }))
}
