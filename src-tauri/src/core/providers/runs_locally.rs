use super::*;

/// Does this model run on this machine, on one endpoint shared by every chat?
///
/// The question behind "why is my second chat waiting". A local runner serves a
/// fixed number of sequences at once — Ollama launches its backend with `-np N`,
/// which is 1 unless `OLLAMA_NUM_PARALLEL` says otherwise — so two chats on the same
/// local model take turns, and the second one's wait is not the app being stuck.
///
/// Lives here, and is used by `pricing` too, because the hard part is the same in
/// both places: an Ollama tag is `name:tag` and looks exactly like `provider:model`,
/// so `gemma4:12b` has no provider prefix and is nonetheless local. A second copy of
/// that rule in the GUI would have to re-learn it.
pub fn runs_locally(model: &str) -> bool {
    let id = model.trim();
    if id.is_empty() {
        return false;
    }
    match resolve(id) {
        // `local` is a fact about the PROVIDER CLASS, and it is not enough on its own.
        // `custom` is registered `local: false` while its own entry says it "is usually
        // pointed at Ollama or an MLX server on loopback" — which is exactly this
        // machine's setup, where `custom:qwen2.5:7b-instruct` reaches Ollama's
        // OpenAI-compatible port. Asking the class alone answered "not local" about a
        // model running on localhost, so the contention notice never appeared for the
        // one configuration that needed it.
        //
        // The address is the honest test, and `is_loopback` is already the app's
        // answer to "is this something the user is running themselves" — the readiness
        // check above uses it for the same reason.
        Some(p) => p.local || is_loopback(&base_url(p.id)),
        // Nothing claims it, so the run path sends it to Ollama.
        None => true,
    }
}

/// The model a turn runs on when nothing has chosen one for it.
///
/// Written out three times before this existed — in `run_agent_inner`, in `/v1/health`,
/// and nowhere at all in the session socket's `ready` frame, which is what made it a
/// bug rather than a duplication. A session that has never had a model explicitly
/// picked stores `""`, `ready` sent that empty string, and the GUI's `if (d.model)`
/// read it as "keep whatever you had" — so opening a new chat inherited the model of
/// the chat before it, and picking a model in one chat silently moved every
/// never-configured chat onto it. Reported 2026-09-01: two chats, one on nemotron and
/// one on gemma, both ended up on gemma.
///
/// The header comment on `/v1/health` already names this failure in its other form:
/// "the app showed gemma4:12b in the header and the composer while running a
/// completely different model … not a broken feature, a confidently wrong label."
/// One copy, so the label and the turn cannot disagree.
pub fn effective_model(settings: &crate::core::settings::Settings) -> String {
    // Asks whether the saved cloud model's own provider is ready, not whether an
    // Anthropic key exists — otherwise a turn with no explicit model runs on the
    // local default however carefully the user picked Codex or Groq.
    if !settings.local_only && settings.cloud_enabled && cloud_model_ready(&settings.cloud_model) {
        settings.cloud_model.clone()
    } else {
        settings.ollama_model.clone()
    }
}

/// Whether the signed-in Codex CLI is available to hand a turn to.
///
/// Two file checks rather than running `codex --version`: this is called to paint
/// the model picker, and a subprocess per repaint is a cost the answer does not
/// need. `CODEX_HOME` is honoured because the CLI honours it.
pub fn codex_ready() -> bool {
    let auth = match std::env::var_os("CODEX_HOME") {
        Some(h) => std::path::PathBuf::from(h).join("auth.json"),
        None => match dirs::home_dir() {
            Some(h) => h.join(".codex").join("auth.json"),
            None => return false,
        },
    };
    if !auth.exists() {
        return false;
    }
    // On PATH, checked the way a shell would.
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("codex").is_file()))
        .unwrap_or(false)
}

/// The endpoint to actually call for `id` — the stored override if there is one,
/// otherwise the registry's default. Always without a trailing slash, so callers
/// can append `/chat/completions` without thinking about it.
pub fn base_url(id: &str) -> String {
    resolve_base(id, crate::core::secrets::get_provider_base_url(id))
}

/// The choice itself, split from the store lookup so it can be tested without
/// writing to the real secrets file — `data_dir()` is the user's own, not a
/// per-test one, and a test that set an override there would change what the
/// app does after the test run.
pub(super) fn resolve_base(id: &str, override_url: Option<String>) -> String {
    if let Some(u) = override_url {
        let u = u.trim().trim_end_matches('/');
        if !u.is_empty() {
            return u.to_string();
        }
    }
    find(id).map(|p| p.base_url.trim_end_matches('/').to_string()).unwrap_or_default()
}

/// Where an OpenAI-compatible turn should post, and what to authenticate with.
///
/// The security-relevant decision in this file. A missing key is only survivable
/// when the endpoint is this machine; anywhere else it must be an error, because
/// the alternative — posting the user's conversation to a stranger with no
/// credential rather than refusing — is the worst outcome available here.
///
/// Takes the key rather than reading it so the rule can be tested without the
/// credential store, which is the user's real one.
pub fn endpoint(base: &str, key: Option<String>, name: &str) -> Result<(String, Option<String>), String> {
    if base.is_empty() {
        return Err(format!("Set a base URL for {name} in Settings → AI Models."));
    }
    let url = format!("{base}/chat/completions");
    match key {
        Some(k) => Ok((url, Some(k))),
        None if is_loopback(base) => Ok((url, None)),
        None => Err(format!("Add an API key for {name} in Settings → AI Models.")),
    }
}

/// Normalize a user-typed endpoint, or say why it can't be used.
///
/// Empty is not an error — it is how the override is cleared — and comes back as
/// `""`. Both rejections are here because the failures they prevent are bad ones to
/// debug: a schemeless URL is refused by reqwest deep inside a turn, in an error
/// that names the URL rather than the setting that produced it, and a base that
/// already ends in `/chat/completions` reaches `…/chat/completions/chat/completions`
/// and 404s with nothing pointing at the cause.
pub fn validate_base_url(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Ok(String::new());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Base URL must start with http:// or https://".into());
    }
    if url.ends_with("/chat/completions") {
        return Err("Base URL should end at /v1 — the app appends /chat/completions.".into());
    }
    Ok(url.to_string())
}

/// The models to offer for `id` — the user's own list if they gave one, otherwise
/// the curated one. A custom endpoint has no curated list because we cannot know
/// what it serves, so without the override it would be reachable and unpickable.
pub fn models_for(id: &str) -> Vec<String> {
    resolve_models(id, crate::core::secrets::get_provider_models(id))
}

/// Split from the store lookup for the same reason as [`resolve_base`].
pub(super) fn resolve_models(id: &str, override_csv: Option<String>) -> Vec<String> {
    if let Some(csv) = override_csv {
        let list: Vec<String> = csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    find(id).map(|p| p.models.iter().map(|s| s.to_string()).collect()).unwrap_or_default()
}

/// Whether a URL points at this machine.
///
/// It decides one thing: whether a missing API key is an error. An endpoint on
/// loopback is something the user is running themselves, and those mostly take no
/// key at all — demanding one would mean inventing a placeholder to satisfy a check
/// that protects nothing, and the failure it produced ("Add an API key") would name
/// the wrong problem.
pub fn is_loopback(url: &str) -> bool {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    // An IPv6 literal is bracketed precisely because it is full of colons, so its
    // host ends at the `]` — splitting on the first `:` like every other form
    // yields "[" and reads as "not this machine".
    let host = match rest.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => rest.split(['/', ':']).next().unwrap_or(""),
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
}

pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}
