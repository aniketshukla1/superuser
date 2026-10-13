use super::*;

pub(super) fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cache identity is `(base_url, model)`, not the model alone: the same id served by
/// a different gateway is a different model, and treating them as one is how a
/// profile from someone's vLLM ends up describing OpenAI's.
pub(super) fn cache_key(base_url: &str, model: &str) -> String {
    format!("{base_url}|{model}")
}

/// Drop what we believe about one model, so the next ask re-discovers it. This is the
/// "re-check" affordance behind the model picker.
pub fn forget(base_url: &str, model: &str) {
    if let Ok(mut c) = cache().lock() {
        c.remove(&cache_key(base_url, model));
    }
}

/// Drop everything. Called when the address of a provider changes, because every
/// profile under it was about the old server.
pub fn forget_all() {
    if let Ok(mut c) = cache().lock() {
        c.clear();
    }
}

// ---------------------------------------------------------------------------
// The one entry point
// ---------------------------------------------------------------------------

/// Everything the loop needs to know about `model`, best source available.
///
/// Never fails and never blocks a turn: an unreachable server, a refused probe or a
/// missing key all fall back to the shipped table and then to [`assumed`], with
/// `source` recording which.
pub async fn resolve(
    settings: &Settings,
    model: &str,
    http: &crate::core::egress::Client,
    infos: Option<&[ModelInfo]>,
) -> ModelProfile {
    let prov = providers::resolve(model);
    let base = prov.map(|p| providers::base_url(p.id)).unwrap_or_default();
    if let Ok(c) = cache().lock() {
        if let Some((at, p)) = c.get(&cache_key(&base, model)) {
            if at.elapsed() < TTL {
                return p.clone();
            }
        }
    }

    let profile = discover(settings, model, http, infos, prov, &base).await;
    // The one line that says which model the loop thinks it is talking to, and on
    // what evidence. Without it, "the effort slider does nothing on this model" and
    // "the profile says this model has no reasoning parameter" are the same symptom.
    crate::trace!(
        "profile",
        "{} source={:?} window={:?} tools={:?} thinking={:?} vision={} tier={:?}",
        profile.id,
        profile.source,
        profile.context_window,
        profile.tools,
        profile.thinking,
        profile.vision,
        profile.tier
    );
    // Only evidence is worth caching. A fallback costs nothing to recompute, and
    // caching it would keep a guess alive for fifteen minutes after the server that
    // could have answered came back up.
    if profile.source.is_evidence() {
        if let Ok(mut c) = cache().lock() {
            c.insert(cache_key(&base, model), (Instant::now(), profile.clone()));
        }
    }
    profile
}

pub(super) async fn discover(
    settings: &Settings,
    model: &str,
    http: &crate::core::egress::Client,
    infos: Option<&[ModelInfo]>,
    prov: Option<&'static providers::Provider>,
    base: &str,
) -> ModelProfile {
    let api_model = providers::model_name(model);
    match prov.map(|p| p.kind) {
        // Ollama: the tag listing the loop already fetched carries everything.
        None | Some(ProviderKind::Ollama) => {
            if let Some(list) = infos {
                if let Some(p) = from_ollama(api_model, list) {
                    return p;
                }
            }
            if let Ok(list) = crate::core::ollama::list_model_infos(settings).await {
                if let Some(p) = from_ollama(api_model, &list) {
                    return p;
                }
            }
            declared(model).unwrap_or_else(|| assumed(model))
        }
        Some(ProviderKind::Anthropic) => {
            if let Some(p) = fetch_anthropic(http, base, api_model).await {
                return p;
            }
            declared(model).unwrap_or_else(|| assumed(model))
        }
        Some(ProviderKind::OpenAiCompatible) => {
            // A model we ship support for needs no probe — we already know, and a
            // probe against a paid endpoint costs the user tokens to learn it twice.
            if let Some(p) = declared(model) {
                return p;
            }
            let Some(p) = prov else { return assumed(model) };
            let key = providers::key_for(p.id);
            let Ok((url, bearer)) = providers::endpoint(base, key, p.name) else {
                return assumed(model);
            };
            match probe_openai(http, &url, bearer.as_deref(), api_model).await {
                Some(m) => {
                    let mut prof = assumed(model);
                    prof.tools = m.tools;
                    prof.reports_usage = m.reports_usage;
                    // Only what the handshake actually established. `assumed` already
                    // says `None`, which is the safe answer for an endpoint that
                    // never got to demonstrate otherwise.
                    prof.thinking = m.accepts_effort.then_some(ThinkingStyle::EffortName);
                    prof.source = ProfileSource::Probed;
                    prof
                }
                None => assumed(model),
            }
        }
        Some(ProviderKind::CodexCli | ProviderKind::ClaudeCli) => declared(model).unwrap_or_else(|| assumed(model)),
    }
}

/// `GET /v1/models`, filtered to the one model we asked about.
pub(super) async fn fetch_anthropic(http: &crate::core::egress::Client, base: &str, model: &str) -> Option<ModelProfile> {
    let key = providers::key_for("anthropic")?;
    let resp = http
        .get(format!("{}/v1/models?limit=1000", base.trim_end_matches('/')))
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .ok()
        .filter(|r| r.status().is_success())?;
    let v: Value = resp.json().await.ok()?;
    v["data"]
        .as_array()?
        .iter()
        .find(|m| m["id"].as_str() == Some(model))
        .and_then(parse_anthropic_model)
}
