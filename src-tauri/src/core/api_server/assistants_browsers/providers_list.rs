use super::super::*;

/// `GET /v1/usage/rollup` — tokens spent today and this week, from this app's own history.
///
/// Not a subscription meter. Superuser has no idea what plan you are on or what any
/// provider has counted against it; this is what the session store can actually vouch
/// for, and it says how much of the history it could account for so the number is never
/// mistaken for a complete one.
pub(in crate::core::api_server) async fn usage_rollup(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let sessions = store(&ctx).list();
    Json(json!(crate::core::usage_rollup::rollup_at(&sessions, now)))
}

/// `POST /v1/settings/effort` — `{ effort }`, one of `core::effort::LEVELS`.
///
/// Rejected rather than coerced when it is off the scale: the composer only ever sends a
/// level it got from `effort_levels`, so anything else is a bug worth seeing rather than
/// a preference worth silently rewriting to the default.
pub(in crate::core::api_server) async fn settings_set_effort(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let want = body["effort"].as_str().unwrap_or("").trim().to_ascii_lowercase();
    if !crate::core::effort::is_level(&want) {
        return Json(json!({
            "ok": false,
            "error": format!("{want:?} is not an effort level"),
            "levels": crate::core::effort::LEVELS,
        }));
    }
    let state = ctx.app.state::<crate::AppState>();
    let max_tokens = {
        let mut s = state.settings.lock().unwrap();
        s.effort = want.clone();
        s.save(&state.data_dir);
        s.max_tokens
    };
    // Report the budget back, because "max" on a 1k reply cap silently means no thinking
    // at all and the slider should be able to say so rather than implying otherwise.
    Json(json!({
        "ok": true,
        "effort": want,
        "thinking_budget": crate::core::effort::budget(&want, max_tokens),
    }))
}

/// `POST /v1/settings/small-model` — `{ model }`, empty to clear.
///
/// A local model only: this slot exists so background work stops taking the chat
/// model's weights, and pointing it at a cloud model would defeat the point twice
/// over (cost, and the loops are meant to stay on-device).
pub(in crate::core::api_server) async fn settings_set_small_model(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let model = body["model"].as_str().unwrap_or("").trim().to_string();
    if crate::core::providers::resolve(&model).is_some() {
        return Json(json!({ "ok": false, "error": "the background model has to be a local one" }));
    }
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.small_model = model.clone();
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "model": model }))
}

/// `POST /v1/settings/embed-model` — `{ model }`.
///
/// Refuses anything that cannot embed. The check is the same one `apply_patch`
/// makes; it is repeated here so the UI gets a reason to show rather than a silent
/// no-op, which is the failure this whole area is being dug out of.
pub(in crate::core::api_server) async fn settings_set_embed_model(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let model = body["model"].as_str().unwrap_or("").trim().to_string();
    if model.is_empty() {
        return Json(json!({ "ok": false, "error": "pick an embedding model" }));
    }
    if !crate::core::ollama::is_embed_model(&model) {
        return Json(json!({
            "ok": false,
            "error": format!("'{model}' isn't an embedding model — memory search would silently fall back to keyword matching"),
        }));
    }
    let state = ctx.app.state::<crate::AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.embed_model = model.clone();
        s.save(&state.data_dir);
    }
    Json(json!({ "ok": true, "model": model }))
}

/// `GET /v1/providers` — the model providers, in the shape the composer's model
/// picker reads (a bare array, not wrapped).
pub(in crate::core::api_server) async fn providers_list() -> Json<Value> {
    let items: Vec<Value> = crate::core::providers::list_info()
        .into_iter()
        .map(|p| {
            let needs_key = !p.local;
            // The address is offered alongside the key for every cloud provider, not
            // just `custom`: pointing an existing one at a gateway or a local proxy is
            // the same operation, and it keeps that provider's curated model ids.
            // `key`, not `name`: ProviderField in api.ts reads `f.key`, so the field
            // this used to emit arrived as undefined and the setup form rendered a
            // row that could never be filled in or saved.
            // Codex asks for nothing: the credential is a `codex login` the CLI
            // holds. A form with an API key box would invite pasting one that is
            // never read, and a base URL box would suggest an endpoint that isn't
            // ours to point anywhere.
            let codex = p.kind.delegates();
            let needs_key = needs_key && !codex;
            let mut fields = vec![];
            if needs_key {
                fields.push(json!({
                    "key": "api_key", "label": "API key", "secret": true, "required": p.id != "custom",
                    "help": "", "placeholder": p.key_hint,
                }));
                fields.push(json!({
                    "key": "base_url", "label": "Base URL", "secret": false, "required": p.id == "custom",
                    "help": "Leave blank for the vendor's own endpoint. Point it at a gateway, a self-hosted server, or a proxy on this machine to send this provider's traffic there instead.",
                    "placeholder": "https://…/v1",
                }));
                if p.id == "custom" {
                    fields.push(json!({
                        "key": "models", "label": "Models", "secret": false, "required": true,
                        "help": "We can't know what a custom endpoint serves, so name the models it should offer.",
                        "placeholder": "gpt-5.5, gpt-5-mini",
                    }));
                }
            }
            json!({
                "name": p.id,
                "title": p.name,
                "needs_key": needs_key,
                "configured": p.ready,
                "fields": fields,
                // Prefilled so the field shows where traffic is actually going —
                // a stale override is invisible otherwise, and reads as the vendor
                // endpoint being broken.
                "values": { "base_url": p.base_url, "models": p.models.join(", ") },
                "suggested_models": p.models,
                "recommended_model": p.models.first().cloned(),
            })
        })
        .collect();
    Json(json!(items))
}

/// The provider routes that need no app state, split out so a test can mount them
/// without a Tauri handle.
///
/// The wiring is the part worth testing. Three of these routes were called by
/// `api.ts` and never registered, so save, verify and remove answered 404 — and
/// the delete path was later written in axum 0.8's `{name}` syntax against axum
/// 0.7, which compiles fine and matches nothing. Neither is a type error; both are
/// a request that doesn't arrive.
///
/// `verify` is not here because it reads app state. What these cover is that the
/// paths and methods are the ones the client actually calls.
pub(in crate::core::api_server) fn provider_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/providers", get(providers_list).post(provider_save))
        .route("/v1/providers/:name", axum::routing::delete(provider_remove))
}
