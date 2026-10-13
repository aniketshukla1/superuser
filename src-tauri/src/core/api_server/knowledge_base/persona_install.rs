use super::super::*;

pub(in crate::core::api_server) fn knowledge(ctx: &Arc<ApiContext>) -> crate::core::knowledge::Knowledge {
    use tauri::Manager;
    crate::core::knowledge::Knowledge::new(&ctx.app.state::<crate::AppState>().memory.vault_path)
}

/// `GET /v1/knowledge` — every topic page, plus the link graph between them.
/// `?q=` filters by keyword.
pub(in crate::core::api_server) async fn knowledge_list(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let k = knowledge(&ctx);
    let needle = q.get("q").map(|s| s.trim()).unwrap_or_default();
    let pages = if needle.is_empty() { k.list() } else { k.search(needle, 50) };
    Json(json!({ "pages": pages, "graph": k.graph() }))
}

/// `DELETE /v1/knowledge/:slug` — forget a page. A knowledge base you can't
/// correct is one you can't trust, same as the memory list.
pub(in crate::core::api_server) async fn knowledge_delete(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<Value> {
    Json(json!({ "ok": knowledge(&ctx).delete(&slug) }))
}

pub(in crate::core::api_server) async fn personas_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    // The GUI's Persona shape is our PersonaListItem, wrapped.
    Json(json!({ "personas": registry(&ctx).list_all() }))
}

/// `POST /v1/personas/install` — take on an assistant somebody else wrote.
///
/// Body is one of `{"dir": "..."}` or `{"git_url": "..."}`. Answers with a consent
/// card per assistant installed plus the refreshed roster, which is what Settings ▸
/// Personas renders under "Installed N — review and enable below".
///
/// Every installed assistant arrives DISABLED. That is not a detail of this
/// handler; it is the feature's safety property, and `personas_install` has the
/// argument for why in full.
///
/// `gallery_slug` is refused. The gallery is browsed behind a cloud sign-in this
/// product deliberately does not have, so honouring the field would mean claiming
/// a source we cannot read — a clear refusal beats a mysterious empty result.
pub(in crate::core::api_server) async fn persona_install(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    use crate::core::personas_install::Source;

    let s = |k: &str| body[k].as_str().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let source = match (s("git_url"), s("dir"), s("gallery_slug")) {
        (Some(u), _, _) => Source::Git(u),
        (_, Some(d), _) => Source::Dir(d),
        (_, _, Some(_)) => {
            return Json(json!({
                "ok": false,
                "error": "the gallery needs a cloud account, and this app doesn't have one — install from a GitHub repository or a folder instead"
            }))
        }
        _ => return Json(json!({ "ok": false, "error": "give a GitHub repository or a folder to install from" })),
    };

    match crate::core::personas_install::install(&dir, source).await {
        Ok(consent) => Json(json!({
            "ok": true,
            "consent": consent,
            "personas": crate::core::personas::PersonaRegistry::load(&dir).list_all(),
        })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/secrets/set-aside` — move a saved-keys file that won't open out of the way, so
/// keys can be saved again. The file is renamed, not deleted, and the reply says where it went.
pub(in crate::core::api_server) async fn secrets_set_aside(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    match crate::core::secrets::set_aside_locked() {
        Ok(path) => {
            crate::core::audit::log(&data_dir, "secrets", "user", "secrets.json", &format!("a store that wouldn't open was set aside as {}", path.display()), true);
            Json(json!({ "ok": true, "kept_at": path.display().to_string() }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/import/package/preview` — `{dir}`: what importing this agent package would
/// bring across, what it would refuse and why, and what has no home here. Writes nothing.
pub(in crate::core::api_server) async fn package_import_preview(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let Some(dir) = body["dir"].as_str().map(str::trim).filter(|d| !d.is_empty()) else {
        return Json(json!({ "ok": false, "error": "Give the package's folder." }));
    };
    let reg = crate::core::personas::PersonaRegistry::load(&data_dir);
    let store = crate::core::skills::SkillStore::load(&data_dir);
    match crate::core::package_import::plan(std::path::Path::new(dir), &reg, &store) {
        Ok(plan) => Json(json!({ "ok": true, "plan": plan })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/import/package` — `{dir, digest}`: import what the preview showed. Refused if the
/// folder no longer matches the digest; every assistant arrives switched off.
pub(in crate::core::api_server) async fn package_import_apply(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = body["dir"].as_str().map(str::trim).unwrap_or("");
    let digest = body["digest"].as_str().unwrap_or("");
    if dir.is_empty() || digest.is_empty() {
        return Json(json!({ "ok": false, "error": "Read the package first, then import what it showed." }));
    }
    match crate::core::package_import::apply(&data_dir, std::path::Path::new(dir), digest) {
        Ok(applied) => Json(json!({
            "ok": true,
            "skills": applied.skills,
            "consent": applied.consent,
            "personas": crate::core::personas::PersonaRegistry::load(&data_dir).list_all(),
        })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// An assistant's default connections, one row per connector it recommends.
///
/// A free function because `json!` cannot take a statement block, and because the
/// detail handler and the write handler must answer this identically — two
/// spellings of "what are the defaults" is how they drift.
pub(in crate::core::api_server) fn default_connections(
    data_dir: &std::path::Path,
    m: &crate::core::personas::PersonaManifest,
) -> Vec<crate::core::persona_connections::DefaultConnection> {
    let recommended: Vec<String> =
        m.recommends.iter().filter(|r| r.kind == "connector").map(|r| r.reference.clone()).collect();
    crate::core::persona_connections::PersonaConnections::load(data_dir).list(&m.id, &recommended, &|c| {
        crate::core::secrets::has_integration_token(crate::integrations::token_key(c))
    })
}

pub(in crate::core::api_server) async fn persona_detail(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let reg = registry(&ctx);
    match reg.manifest(&id) {
        Some(m) => Json(json!({
            "id": m.id, "name": m.name, "icon": m.icon, "tagline": m.tagline,
            "description": m.description, "family": m.family, "workspace": m.workspace,
            "tools": m.tools, "builtin": m.builtin, "messaging": m.messaging,
            "connectors": m.connectors, "mode": m.default_permission_mode,
            "needs_capable_model": m.needs_capable_model, "skills": m.skills,
            "instructions": m.system_prompt,
            "recommends": m.recommends,
            "subagents": m.subagents,
            // Who this one may hand work to. A grant, so it lives in the
            // registry's state rather than in a manifest compiled into the binary.
            "asks": reg.asks(&m.id),
            // Where its turns run: an address when a remote agent does
            // them. Whether a token is saved, never the token itself.
            "remote": reg.remote(&m.id),
            "remote_token_set": reg.remote(&m.id).is_some() && crate::core::secrets::get_agent_token(&m.id).is_some(),
            // Three fields the client's `PersonaDetail` declares and this handler
            // never sent. `default_connections` is the one that mattered: it is typed
            // as a required array and dereferenced with `.length`, so its absence did
            // not degrade the screen, it unmounted it. `mode` stays for anything
            // still reading the old key.
            "enabled": reg.is_enabled(&m.id),
            "default_permission_mode": m.default_permission_mode,
            "default_connections": default_connections(&data_dir, m),
        })),
        None => Json(json!({ "error": format!("unknown persona: {id}") })),
    }
}

/// What a persona recommends connecting — the GUI's per-session connections drawer.
pub(in crate::core::api_server) async fn persona_connections(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let recs = registry(&ctx).manifest(&id).map(|m| m.recommends.clone()).unwrap_or_default();
    let items: Vec<Value> = recs
        .iter()
        .map(|r| {
            let connected = r.kind == "connector"
                && crate::core::secrets::has_integration_token(crate::integrations::token_key(&r.reference));
            json!({ "kind": r.kind, "ref": r.reference, "reason": r.reason, "tier": r.tier, "connected": connected })
        })
        .collect();
    Json(json!({ "connections": items }))
}
