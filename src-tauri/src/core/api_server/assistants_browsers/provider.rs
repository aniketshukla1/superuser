use super::super::*;

/// `POST /v1/providers` — save a provider's key and/or where its traffic goes.
///
/// Every field is optional and only a field that is present is written, so saving
/// an endpoint does not clear the key that was already there.
pub(in crate::core::api_server) async fn provider_save(Json(body): Json<Value>) -> Json<Value> {
    let name = body["name"].as_str().unwrap_or("").trim().to_string();
    if crate::core::providers::find(&name).is_none() {
        return Json(json!({ "ok": false, "error": format!("unknown provider: {name}") }));
    }
    let field = |k: &str| body["fields"][k].as_str().map(|s| s.trim().to_string());

    if let Some(url) = field("base_url") {
        match crate::core::providers::validate_base_url(&url) {
            Ok(u) if u.is_empty() => { crate::core::secrets::clear_provider_base_url(&name); }
            Ok(u) => { crate::core::secrets::set_provider_base_url(&name, &u); }
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    if let Some(models) = field("models") {
        if models.is_empty() {
            crate::core::secrets::clear_provider_models(&name);
        } else {
            crate::core::secrets::set_provider_models(&name, &models);
        }
    }
    // A blank key is "leave it alone", not "delete it" — the form sends every field
    // it rendered, and the key input is empty on a revisit because it is secret.
    // Treating that as a clear would wipe the key every time an endpoint was edited.
    if let Some(key) = field("api_key") {
        if !key.is_empty() {
            if name == "anthropic" {
                crate::core::secrets::set_cloud_key(&key);
            } else {
                crate::core::secrets::set_provider_key(&name, &key);
            }
        }
    }
    Json(json!({
        "ok": true,
        "provider": name,
        "recommended_model": crate::core::providers::models_for(&name).first().cloned(),
    }))
}

/// `POST /v1/providers/verify` — a live read-only check of what was typed, without
/// saving any of it. The user pressed Test; a save that "succeeds" and then fails
/// on the first turn is the thing this exists to prevent.
pub(in crate::core::api_server) async fn provider_verify(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let name = body["name"].as_str().unwrap_or("").trim().to_string();
    let Some(p) = crate::core::providers::find(&name) else {
        return Json(json!({ "ok": false, "error": format!("unknown provider: {name}") }));
    };
    let field = |k: &str| body["fields"][k].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    // What was typed wins over what is stored, so Test checks the pending edit
    // rather than the last thing that was saved.
    let base = match field("base_url") {
        Some(u) => match crate::core::providers::validate_base_url(&u) {
            Ok(u) => u,
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        },
        None => crate::core::providers::base_url(&name),
    };
    let key = field("api_key").or_else(|| {
        if name == "anthropic" { crate::core::secrets::get_cloud_key() } else { crate::core::secrets::get_provider_key(&name) }
    });

    // Codex has nothing to reach out to: there is no endpoint of ours in the path
    // and no key to check. "Configured" means the CLI is installed and signed in,
    // which is the same question `codex_ready` answers for the picker.
    if p.kind == crate::core::providers::ProviderKind::CodexCli {
        return if crate::core::providers::codex_ready() {
            Json(json!({ "ok": true }))
        } else {
            Json(json!({ "ok": false, "error": "Codex isn't signed in on this machine. Run `codex login`, then test again." }))
        };
    }
    // The same question for Claude Code: installed, and signed in with `claude`.
    if p.kind == crate::core::providers::ProviderKind::ClaudeCli {
        return if crate::core::claude_cli::ready() {
            Json(json!({ "ok": true }))
        } else {
            Json(json!({ "ok": false, "error": "Claude Code isn't installed or signed in. Run `claude`, sign in with /login, then test again." }))
        };
    }

    let http = crate::core::egress::client("providers");
    let (url, req) = match p.kind {
        // Returned above — it has no endpoint to build a request against. Spelled
        // out rather than folded into a `_` arm so that a future kind with a real
        // endpoint fails to compile here instead of silently verifying nothing.
        crate::core::providers::ProviderKind::CodexCli => unreachable!("Codex is answered before this match"),
        crate::core::providers::ProviderKind::ClaudeCli => unreachable!("Claude Code is answered before this match"),
        crate::core::providers::ProviderKind::Ollama => {
            let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
            let settings = crate::core::settings::Settings::load(&data_dir);
            let u = format!("{}/api/tags", settings.ollama_url.trim_end_matches('/'));
            (u.clone(), http.get(&u))
        }
        crate::core::providers::ProviderKind::Anthropic => {
            let Some(k) = key else { return Json(json!({ "ok": false, "error": "Add an API key first." })) };
            let u = format!("{}/v1/models", if base.is_empty() { "https://api.anthropic.com".into() } else { base });
            (u.clone(), http.get(&u).header("x-api-key", k).header("anthropic-version", "2023-06-01"))
        }
        crate::core::providers::ProviderKind::OpenAiCompatible => {
            if base.is_empty() {
                return Json(json!({ "ok": false, "error": "Set a base URL first." }));
            }
            if key.is_none() && !crate::core::providers::is_loopback(&base) {
                return Json(json!({ "ok": false, "error": "Add an API key first." }));
            }
            let u = format!("{base}/models");
            let mut r = http.get(&u);
            if let Some(k) = key {
                r = r.header("authorization", format!("Bearer {k}"));
            }
            (u, r)
        }
    };
    match req.send().await {
        Ok(r) if r.status().is_success() => Json(json!({ "ok": true })),
        // The status is the whole diagnosis here — 401 is a bad key and 404 is a
        // wrong address, and they are fixed in different fields.
        Ok(r) => Json(json!({ "ok": false, "error": format!("{} answered {}", url, r.status()) })),
        Err(e) => Json(json!({ "ok": false, "error": format!("{url} unreachable: {e}") })),
    }
}

/// `DELETE /v1/providers/:name` — forget everything stored for a provider: the
/// key, the endpoint, and the model list, so "Remove" leaves nothing behind that
/// would quietly still be in force.
pub(in crate::core::api_server) async fn provider_remove(axum::extract::Path(name): axum::extract::Path<String>) -> Json<Value> {
    if crate::core::providers::find(&name).is_none() {
        return Json(json!({ "ok": false, "error": format!("unknown provider: {name}") }));
    }
    if name == "anthropic" {
        crate::core::secrets::clear_cloud_key();
    }
    crate::core::secrets::clear_provider_key(&name);
    crate::core::secrets::clear_provider_base_url(&name);
    crate::core::secrets::clear_provider_models(&name);
    Json(json!({ "ok": true }))
}
