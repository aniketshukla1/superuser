use super::super::*;

/// Pull a connected app's content into the Memory Tree.
pub(in crate::core::api_server) async fn connector_sync_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let engine = crate::core::memory::MemoryEngine::new(state.memory.vault_path.clone());
    let dir = state.data_dir.clone();
    match crate::integrations::sync_one(&name, &engine).await {
        Ok(out) => {
            crate::core::audit::log(&dir, "sync", "user", &name, &format!("{} new", out.created), true);
            Json(json!({ "ok": true, "created": out.created, "titles": out.titles }))
        }
        Err(e) => {
            crate::core::audit::log(&dir, "sync", "user", &name, &e, false);
            Json(json!({ "ok": false, "error": e }))
        }
    }
}

/// `GET /v1/profile` — the form, everything learned since, and what was dropped.
///
/// Provisional facets ship alongside the Active ones on purpose. A store that
/// silently decides what to believe about someone, and only shows them the
/// conclusions, is the version of this feature that people find creepy; showing
/// what it is considering — and the words that put it there — is what makes
/// pin and forget mean something.
pub(in crate::core::api_server) async fn profile_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let dir = state.data_dir.clone();
    let enabled = state.settings.lock().unwrap().profile_learning_enabled;
    let learned = crate::core::profile::Learned::load(&dir);
    Json(json!({
        "form": learned.form(),
        "facets": learned.facets(),
        "forgotten": learned.forgotten(),
        "learning_enabled": enabled,
        "path": crate::core::profile::profile_md_path(&state.memory.vault_path).to_string_lossy(),
    }))
}

/// `POST /v1/profile/form` — replace the fields the user typed about themselves.
pub(in crate::core::api_server) async fn profile_save_form(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let form: crate::core::profile::Profile = match serde_json::from_value(body) {
        Ok(f) => f,
        Err(e) => return Json(json!({ "ok": false, "error": format!("could not read the profile: {e}") })),
    };
    let state = ctx.app.state::<crate::AppState>();
    crate::core::profile::save(&state.data_dir, &form);
    // An edit to your own name should apply to the conversation you're in, so the
    // per-session freeze is dropped here rather than waiting for the next session.
    crate::core::profile::thaw_all();
    let learned = crate::core::profile::Learned::load(&state.data_dir);
    let _ = learned.write_profile_md(&state.memory.vault_path);
    Json(json!({ "ok": true, "form": learned.form(), "facets": learned.facets() }))
}

/// `POST /v1/profile/learning` — `{ enabled }`. Turning it off stops new
/// observations being made; what has already been learned keeps applying, because
/// deleting someone's profile is a different request from switching the learner off.
pub(in crate::core::api_server) async fn profile_set_learning(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.profile_learning_enabled = enabled;
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "learning_enabled": enabled }))
}

/// The class/key/value triple every verdict below is addressed by.
pub(in crate::core::api_server) fn facet_ref(body: &Value) -> Result<(crate::core::profile::Class, String, String), String> {
    let class = crate::core::profile::Class::parse(body["class"].as_str().unwrap_or(""))
        .ok_or_else(|| "unknown facet class".to_string())?;
    let key = body["key"].as_str().unwrap_or("").trim().to_string();
    let value = body["value"].as_str().unwrap_or("").trim().to_string();
    if key.is_empty() || value.is_empty() {
        return Err("a facet is addressed by class, key and value".into());
    }
    Ok((class, key, value))
}

/// `POST /v1/profile/pin` — `{ class, key, value, pinned }`. Pinning shields a
/// belief from decay; unpinning lets it fade again from the evidence it has.
pub(in crate::core::api_server) async fn profile_pin(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let (class, key, value) = match facet_ref(&body) {
        Ok(v) => v,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let state = ctx.app.state::<crate::AppState>();
    let mut learned = crate::core::profile::Learned::load(&state.data_dir);
    learned.set_pinned(class, &key, &value, body["pinned"].as_bool().unwrap_or(true));
    crate::core::profile::thaw_all();
    let _ = learned.write_profile_md(&state.memory.vault_path);
    Json(json!({ "ok": true, "facets": learned.facets() }))
}

/// `POST /v1/profile/forget` — `{ class, key, value, forget? }`. Drops a belief
/// and blocks it from being learned again; `forget: false` lifts the block.
pub(in crate::core::api_server) async fn profile_forget(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let (class, key, value) = match facet_ref(&body) {
        Ok(v) => v,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let state = ctx.app.state::<crate::AppState>();
    let mut learned = crate::core::profile::Learned::load(&state.data_dir);
    if body["forget"].as_bool().unwrap_or(true) {
        learned.forget(class, &key, &value);
    } else {
        learned.unforget(class, &key, &value);
    }
    crate::core::profile::thaw_all();
    let _ = learned.write_profile_md(&state.memory.vault_path);
    Json(json!({ "ok": true, "facets": learned.facets(), "forgotten": learned.forgotten() }))
}

/// `GET /v1/watches` — standing watches plus what the loop has been doing.
///
/// Activity rides along with the list because the two are only meaningful
/// together: a watch with no activity has either never fired or is broken, and the
/// UI shouldn't have to make two calls to tell those apart.
pub(in crate::core::api_server) async fn watches_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let dir = state.data_dir.clone();
    let enabled = state.settings.lock().unwrap().subconscious_enabled;
    let s = crate::core::subconscious::Subconscious::load(&dir);
    Json(json!({
        "watches": s.watches,
        "activity": s.activity().into_iter().take(50).collect::<Vec<_>>(),
        "enabled": enabled,
    }))
}

/// `POST /v1/watches` — add one. `{ question, sources?, writes? }`
pub(in crate::core::api_server) async fn watch_create(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    if question.is_empty() {
        return Json(json!({ "ok": false, "error": "a watch needs a question" }));
    }
    let sources: Vec<String> = body["sources"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    let w = crate::core::subconscious::Subconscious::load(&dir)
        .create(crate::core::subconscious::Watch::new(question, sources, body["writes"].as_bool().unwrap_or(false)));
    Json(json!({ "ok": true, "watch": w }))
}

/// `PATCH /v1/watches/:id` — currently just the enabled toggle.
pub(in crate::core::api_server) async fn watch_patch(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let mut s = crate::core::subconscious::Subconscious::load(&dir);
    let ok = match body["enabled"].as_bool() {
        Some(v) => s.set_enabled(&id, v),
        None => false,
    };
    Json(json!({ "ok": ok }))
}

pub(in crate::core::api_server) async fn watch_delete(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({ "ok": crate::core::subconscious::Subconscious::load(&dir).delete(&id) }))
}
