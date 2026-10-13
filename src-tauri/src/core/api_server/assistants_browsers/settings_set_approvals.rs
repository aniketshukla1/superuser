use super::super::*;

/// `POST /v1/settings/stall` — how long a model may say nothing before its turn ends.
pub(in crate::core::api_server) async fn settings_set_stall(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let mut s = state.settings.lock().unwrap();
    let mut patch = serde_json::Map::new();
    for key in ["stall_seconds", "stall_seconds_local"] {
        if let Some(v) = body.get(key).and_then(|v| v.as_u64()) {
            patch.insert(key.into(), json!(v));
        }
    }
    s.apply_patch(&Value::Object(patch));
    s.save(&state.data_dir);
    Json(json!({ "ok": true, "stall_seconds": s.stall_seconds, "stall_seconds_local": s.stall_seconds_local }))
}

/// `POST /v1/settings/audit-retention` — how many days of trail to keep; 0 keeps all.
///
/// Applied at once, not at next launch: a person who shortens it is looking at the
/// page and should see the result. What was removed is itself a row in the trail.
pub(in crate::core::api_server) async fn settings_set_audit_retention(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let days = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&json!({ "audit_retention_days": body["days"].as_u64().unwrap_or(0) }));
        s.save(&state.data_dir);
        s.audit_retention_days
    };
    crate::core::audit::log(
        &state.data_dir,
        "settings",
        "user",
        "audit retention",
        &if days == 0 { "keep everything".to_string() } else { format!("keep {days} days") },
        true,
    );
    let dir = state.data_dir.clone();
    let removed = tokio::task::spawn_blocking(move || crate::core::audit::apply_retention(&dir, days)).await.unwrap_or(0);
    Json(json!({ "ok": true, "audit_retention_days": days, "removed": removed }))
}

/// `POST /v1/settings/nav-layout` — group the sidebar by assistant, or don't.
///
/// An unknown value is refused rather than coerced. The other two settings here
/// have an obvious sensible reading for junk input; this one does not, and
/// silently storing "grouped" when the client asked for something else would
/// leave the sidebar disagreeing with the toggle that set it.
pub(in crate::core::api_server) async fn settings_set_nav_layout(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let want = body["nav_layout"].as_str().unwrap_or("").trim().to_lowercase();
    if want != "flat" && want != "grouped" {
        return Json(json!({ "ok": false, "error": format!("{want:?} is not a sidebar layout") }));
    }
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.nav_layout = want.clone();
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "nav_layout": want }))
}

/// `POST /v1/settings/approvals` — the two switches that decide how much happens
/// without the user looking.
///
/// One route for both, because they are one decision from the user's side ("how
/// much do I want to be asked?") and a card that saves two fields through two
/// endpoints can leave them disagreeing when the second call fails.
///
/// Absent fields are left alone rather than defaulted to false: a client that
/// knows about one switch must not silently turn off the other.
pub(in crate::core::api_server) async fn settings_set_approvals(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let (auto_approve, read_only) = {
        let mut s = state.settings.lock().unwrap();
        if let Some(v) = body["auto_approve"].as_bool() {
            s.auto_approve = v;
        }
        if let Some(v) = body["auto_allow_read_only_commands"].as_bool() {
            s.auto_allow_read_only_commands = v;
        }
        s.save(&state.data_dir);
        (s.auto_approve, s.auto_allow_read_only_commands)
    };
    crate::core::audit::log(
        &state.data_dir,
        "approve",
        "user",
        "settings",
        &format!("auto_approve={auto_approve} read_only_commands={read_only}"),
        true,
    );
    Json(json!({
        "ok": true,
        "auto_approve": auto_approve,
        "auto_allow_read_only_commands": read_only,
    }))
}

/// `POST /v1/settings/default-model` — Settings' "Make default" button.
///
/// The button has always been there; this route was not, so every click 404'd and the
/// default silently stayed wherever the fallback had put it. That is what "I can't change
/// the model" looked like from the outside.
///
/// A cloud id (`provider:model`) sets the cloud default, a bare id the local one, so the
/// choice survives the local/cloud switch rather than being overwritten by it.
pub(in crate::core::api_server) async fn settings_set_default_model(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let model = body["model"].as_str().unwrap_or("").trim().to_string();
    if model.is_empty() {
        return Json(json!({ "ok": false, "error": "no model given" }));
    }
    let state = ctx.app.state::<crate::AppState>();
    let is_cloud = crate::core::providers::resolve(&model).is_some();
    {
        let mut s = state.settings.lock().unwrap();
        if is_cloud {
            s.cloud_model = model.clone();
        } else {
            s.ollama_model = model.clone();
        }
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "model": model }))
}

/// `GET /v1/voice/devices` — the composer's microphone menu.
///
/// Read live every time, never cached: an iPhone mic appears when the phone is on the
/// desk and is gone when it leaves, and a menu listing last week's devices is worse
/// than no menu.
pub(in crate::core::api_server) async fn voice_devices(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let (selected, hold) = {
        let s = state.settings.lock().unwrap();
        (s.mic_device.clone(), s.mic_hold_to_record)
    };
    let devices = crate::voice::list_input_devices();
    // A device that was chosen and is now unplugged: the menu still shows the choice as
    // the effective one would be wrong, because recording will fall back to the default.
    let selected_present = selected.is_empty() || devices.iter().any(|d| d.name == selected);
    Json(json!({
        "devices": devices,
        "selected": selected,
        "selected_present": selected_present,
        "hold_to_record": hold,
    }))
}

/// `POST /v1/voice/device` — `{ device?, hold_to_record? }`. Empty device = system default.
///
/// The device is NOT validated against the current list. You can legitimately choose a
/// mic that is unplugged right now and expect it to be used when you plug it back in;
/// `preferred_device` falls back to the default in the meantime.
pub(in crate::core::api_server) async fn voice_set_device(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let (device, hold) = {
        let mut s = state.settings.lock().unwrap();
        if let Some(d) = body["device"].as_str() {
            s.mic_device = d.trim().to_string();
        }
        if let Some(h) = body["hold_to_record"].as_bool() {
            s.mic_hold_to_record = h;
        }
        s.save(&state.data_dir);
        (s.mic_device.clone(), s.mic_hold_to_record)
    };
    Json(json!({ "ok": true, "device": device, "hold_to_record": hold }))
}
