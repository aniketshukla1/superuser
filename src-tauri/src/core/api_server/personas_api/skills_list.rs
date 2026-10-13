use super::super::*;

/// `GET /v1/skills` — the Skills tab. Built-in deliverable templates plus anything
/// the user installed under data_dir/skills/.
/// `POST /v1/skills/preview` — fetch a skill and show what it says. Writes nothing.
///
/// Separate from installing on purpose: a skill is instructions the assistant will
/// follow, so the user gets to read the actual text before it can steer anything.
pub(in crate::core::api_server) async fn skill_preview(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    match crate::core::skills_install::preview(&dir, body["source"].as_str().unwrap_or("")).await {
        Ok(p) => Json(json!({ "ok": true, "preview": p })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/skills/install` — write a preview the user has approved.
///
/// Takes the reviewed preview back, rather than re-fetching from the source: what
/// lands on disk is then exactly the text that was on screen, with no window in
/// which the remote content could change.
pub(in crate::core::api_server) async fn skill_install(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let Ok(preview) = serde_json::from_value::<crate::core::skills_install::Preview>(body["preview"].clone()) else {
        return Json(json!({ "ok": false, "error": "install needs the preview that was approved" }));
    };
    match crate::core::skills_install::install(&dir, &preview) {
        Ok(path) => Json(json!({ "ok": true, "name": preview.name, "path": path.to_string_lossy() })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `DELETE /v1/skills/:name` — remove an installed skill. Built-ins are compiled in
/// and have no directory, so they can't be removed this way.
pub(in crate::core::api_server) async fn skill_uninstall(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let ok = crate::core::skills_install::uninstall(&dir, &name);
    if ok {
        crate::core::skills::SkillStore::forget(&dir, &name);
    }
    Json(json!({ "ok": ok }))
}

/// Switch a skill on or off everywhere — the Skills page's switch. Off
/// keeps it installed and listed, and offers it to no assistant. Editing a skill's text isn't
/// built: a body that asks for it is told so rather than answered `ok` having changed nothing.
pub(in crate::core::api_server) async fn skill_update(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(switch_skill(&dir, &name, &body))
}

/// What the switch does, apart from the route, so it is tested without a running app.
pub(in crate::core::api_server) fn switch_skill(dir: &std::path::Path, name: &str, body: &Value) -> Value {
    if body.get("description").is_some() || body.get("instructions").is_some() {
        return json!({ "ok": false, "error": "Editing a skill here isn't built yet. Its text is its SKILL.md." });
    }
    let Some(on) = body["enabled"].as_bool() else {
        return json!({ "ok": false, "error": "say `enabled`: true or false" });
    };
    let mut store = crate::core::skills::SkillStore::load(dir);
    match store.set_on(name, on) {
        Ok(()) => {
            crate::core::audit::log(dir, "skill", "user", name, if on { "switched on" } else { "switched off" }, true);
            json!({ "ok": true })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

pub(in crate::core::api_server) async fn skills_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let store = crate::core::skills::SkillStore::load(&dir);
    let skills: Vec<Value> = store
        .names()
        .into_iter()
        .filter_map(|n| store.get(&n).cloned())
        .map(|s| {
            // `path` is None for the ones we ship, which is exactly what marks a
            // skill as built-in rather than something the user installed.
            let builtin = s.path.is_none();
            json!({
                "name": s.name,
                "description": s.description,
                "instructions": s.instructions,
                "scope": "global",
                "source": if builtin { "builtin" } else { "local" },
                "enabled": store.is_on(&s.name),
                "path": s.path.unwrap_or_default(),
            })
        })
        .collect();
    Json(json!({ "skills": skills }))
}
