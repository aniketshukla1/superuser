use super::super::*;

pub(in crate::core::api_server) async fn settings_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let data_dir = state.data_dir.clone();

    let mut models: Vec<String> = Vec::new();
    let mut labels = serde_json::Map::new();
    // Whatever Ollama actually has pulled — the local-first default.
    let local_infos = crate::core::ollama::list_model_infos(&settings).await.unwrap_or_default();
    for m in &local_infos {
        let profile = crate::core::model_profile::from_ollama(&m.name, &local_infos);
        // An id the loop refuses is worse than a shorter list. Tool support is now
        // checked before a turn starts, so offering a chat-only model — an embedding
        // model, most often, which this list used to include — is offering a
        // guaranteed error. `from_ollama` is also what decides that an Ollama too old
        // to report capabilities means "unknown", not "no", so the list cannot go
        // empty on those installs.
        if profile.as_ref().is_some_and(|p| !p.can_run_a_turn()) {
            continue;
        }
        // The real window, discovered rather than assumed. Shown because the context
        // setting is a memory choice the user can only make well if they know what
        // the model can hold — 16k is the shipped default and gemma4 holds 262k.
        let window = profile
            .as_ref()
            .and_then(|p| p.context_window)
            .map(|w| format!(" · {} context", crate::core::model_profile::human_tokens(w)))
            .unwrap_or_default();
        labels.insert(m.name.clone(), json!(format!("{} · Ollama{window}", m.name)));
        models.push(m.name.clone());
    }
    // Curated cloud models, but only for providers the user has configured — offering
    // ids that can't run is the failure mode their comment warns about. `ready`
    // rather than `has_key` because an endpoint on loopback is configured once it
    // has an address: it has no key to give, and filtering on one hid it entirely.
    for p in crate::core::providers::list_info().into_iter().filter(|p| !p.local && p.ready) {
        // Codex is not a model this app runs — it is another agent this app hands
        // the turn to, tools and all. The picker is the only place a user makes
        // that choice, so it is the place that has to say so.
        let delegates = p.kind.delegates();
        for m in p.models {
            let id = format!("{}:{}", p.id, m);
            let label = if delegates {
                // Named for the agent that actually runs the turn.
                let agent = if p.kind == crate::core::providers::ProviderKind::ClaudeCli { "Claude Code" } else { "Codex" };
                format!("{m} · {} — runs {agent}'s own agent", p.name)
            } else {
                format!("{m} · {}", p.name)
            };
            labels.insert(id.clone(), json!(label));
            models.push(id);
        }
    }
    // Whether the saved cloud model can run is a question about *its own* provider.
    // This used to ask `has_cloud_key()` — the Anthropic key — for every provider
    // alike, so choosing Groq, Codex or a custom endpoint wrote the id to
    // `cloud_model` and then read `ollama_model` back on the next paint: the picker
    // silently reverted to a local model and the turn ran on something the user had
    // not chosen. It answered wrongly in both directions, since an Anthropic key
    // also made an unusable cloud model look configured.
    let cloud_ok = !settings.local_only
        && settings.cloud_enabled
        && crate::core::providers::cloud_model_ready(&settings.cloud_model);
    let configured = if cloud_ok { settings.cloud_model.clone() } else { settings.ollama_model.clone() };
    // Never offer a model that can't run: if the configured one isn't installed
    // (or keyed), fall back to something that is, rather than letting the user
    // pick an id every turn will 404 on. Rank the fallback the same way the turn
    // itself does — `models.first()` is Ollama's listing order, which put a 19.7 GB
    // no-tools model in the default badge on a 16 GB machine.
    let model = if models.contains(&configured) {
        configured.clone()
    } else {
        crate::core::ollama::resolve_chat_model_info(&local_infos, &configured)
            .or_else(|| models.first().cloned())
            .unwrap_or(configured)
    };

    Json(json!({
        // The provider actually serving this model, not "anthropic" for everything
        // that isn't Ollama — that label predates there being other cloud providers,
        // and it names the wrong vendor in the UI for every one of them.
        "provider": if cloud_ok {
            crate::core::providers::resolve(&model).map(|p| p.id).unwrap_or("anthropic")
        } else {
            "ollama"
        },
        "model": model,
        // The local model background work uses, when it differs from the chat one.
        "small_model": settings.small_model,
        // What memory search runs on, and which installed models can serve it. Sent
        // together because the picker is useless without both: the field was absent
        // from this payload entirely, which is why the UI had no control for the one
        // setting the whole memory system rests on.
        "embed_model": settings.embed_model,
        // From the raw Ollama listing, NOT from `models`: that list has already had
        // every model that can't run a turn removed, and an embedding model is
        // exactly that — so filtering it for embedding models is guaranteed to
        // return nothing, and the picker would always be empty.
        "embed_models": local_infos.iter().map(|m| m.name.clone())
            .filter(|n| crate::core::ollama::is_embed_model(n)).collect::<Vec<String>>(),
        "models": models,
        "has_key": crate::core::secrets::has_cloud_key(),
        "model_ready": !models.is_empty(),
        "source": "store",
        "onboarded": settings.onboarded,
        // The shape the client's `SurfaceVisibility` declares — `{cowork, chat, code}`.
        // What went out before was a different concept's keys entirely
        // (`automations`, `inbox`, `connectors`, …), so `s.surfaces` was truthy,
        // App.tsx adopted it, and every field it actually reads came back
        // undefined — including `cowork`, which is supposed to be always visible.
        "surfaces": surfaces_payload(&settings),
        "scratch_base": if settings.scratch_base.trim().is_empty() {
            data_dir.join("sessions").to_string_lossy().to_string()
        } else {
            settings.scratch_base.clone()
        },
        "secrets_path": data_dir.join("secrets.json").to_string_lossy(),
        // Group sessions under the assistant that ran them: assistants are the
        // organising idea, and a flat list is just truncated prompts in a column.
        "nav_layout": settings.nav_layout,
        // Read from the crate so the About row can never drift from the real build.
        "version": env!("CARGO_PKG_VERSION"),
        "model_labels": labels,
        // The composer's effort slider. Normalised on the way out so a hand-edited or
        // pre-existing settings file can never put the slider in a position that is not
        // on the scale.
        "effort": crate::core::effort::normalize(&settings.effort),
        "effort_levels": crate::core::effort::LEVELS,
        // Which leading `x:` on a model id is a PROVIDER rather than part of the name.
        // The client needs this to shorten "anthropic:claude-opus-4-8" without also
        // butchering "gemma4:12b", and it is the server's list, so the server sends it.
        "provider_prefixes": crate::core::providers::PROVIDERS.iter().map(|p| p.id).collect::<Vec<_>>(),
        // How much runs without a card. Both off by default; both are the user's to
        // turn on, which is why they are reported here rather than inferred anywhere.
        "auto_approve": settings.auto_approve,
        "auto_allow_read_only_commands": settings.auto_allow_read_only_commands,
        // Display preferences the GUI reads back on every load. They were reported
        // as constants here while the routes that set them did not exist, so the
        // panel showed a value nothing could change.
        "sessions_peek": settings.sessions_peek,
        "context_bar": settings.context_bar,
        // What every assistant is told, how long a quiet model is
        // given, and how much of the trail is kept.
        "standing_instructions": settings.standing_instructions,
        "standing_instructions_max": crate::core::settings::STANDING_INSTRUCTIONS_MAX,
        "stall_seconds": settings.stall_seconds,
        "stall_seconds_local": settings.stall_seconds_local,
        "audit_retention_days": settings.audit_retention_days,
        // The assistants' own browser.
        "browser_enabled": settings.browser_enabled,
        "browser_allow_loopback": settings.browser_allow_loopback,
        "browser_headed": settings.browser_headed,
        // Phase 5: views an assistant draws for itself.
        "generative_ui": settings.generative_ui,
    }))
}

/// `POST /v1/settings/generative-ui` — `{"on": bool}`: may assistants draw a view of their
/// own in the conversation.
///
/// Off by default. The frame is sealed — its own origin, no network, no reach into this
/// app — and it still renders markup a model wrote, which is a thing to agree to.
pub(in crate::core::api_server) async fn settings_set_generative_ui(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let on = body["on"].as_bool().unwrap_or(false);
    let mut s = state.settings.lock().unwrap();
    s.apply_patch(&json!({ "generative_ui": on }));
    s.save(&state.data_dir);
    Json(json!({ "ok": true, "generative_ui": s.generative_ui }))
}
