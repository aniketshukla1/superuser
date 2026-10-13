use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// base64 images attached to this turn (for vision models). Skipped on the
    /// wire when empty so plain text turns are unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    /// base64 PDF documents for Claude's native PDF reader (scanned PDFs). Never
    /// sent to Ollama (those turns route to the cloud).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<String>,
}

pub enum Provider {
    Ollama,
    Claude,
}

/// What a call is *for*, which is what should decide the model — not whether the
/// caller happens to think a frontier model is worth it.
///
/// The distinction that matters is who is waiting. A user's turn should get the
/// best model available. The bookkeeping the app does on its own — triaging what
/// changed, summarizing turns that fell out of the window, the nightly
/// consolidation, naming a conversation — should not, and until this existed it
/// did: every background call resolved to `ollama_model`, the same model the user
/// chats with. On a machine running a 7B+ chat model that means the subconscious
/// competes for the GPU with the thing it is supposed to be serving, every tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workload {
    /// The user is waiting on this answer. The flag is the caller's judgement about
    /// whether this particular one is worth the frontier model.
    Foreground(bool),
    /// Work the user didn't ask for and isn't watching, of the kind a small model
    /// does well: classify this, summarize that, name a conversation. Never goes to
    /// the cloud — billing someone for a background loop is how always-on
    /// assistants become unaffordable — and prefers the small model when one is
    /// configured.
    Background,
    /// Background work that still needs the chat model's capability.
    ///
    /// The two passes that rewrite memory — the nightly consolidation and the
    /// self-evolution review — demand an exact output format, and their answers are
    /// discarded when it isn't met. A smaller model reasons about the change
    /// correctly and then fails to emit it, so every pass comes back empty. Cheap
    /// is the wrong trade when the alternative is a feature that silently never
    /// works; these stay local but keep the full model.
    BackgroundCapable,
}

/// Which provider handles a request, and which model it should ask for.
///
/// Returning the model too is the point: "Ollama" was never a complete answer, and
/// resolving the name inside the transport meant every caller silently got the
/// chat model.
pub fn route(settings: &Settings, workload: Workload) -> (Provider, String) {
    let cloud_ok = !settings.local_only && settings.cloud_enabled && secrets::has_cloud_key();
    match workload {
        Workload::Foreground(true) if cloud_ok => (Provider::Claude, settings.cloud_model.clone()),
        Workload::Foreground(_) | Workload::BackgroundCapable => (Provider::Ollama, local_chat_model(settings)),
        // Empty is the default, and means "no separate small model" — so an install
        // that never touches the setting behaves exactly as it did before.
        Workload::Background => {
            let small = settings.small_model.trim();
            if small.is_empty() {
                (Provider::Ollama, local_chat_model(settings))
            } else {
                (Provider::Ollama, small.to_string())
            }
        }
    }
}

/// The local model this app is *actually* running on.
///
/// This existed as `settings.ollama_model`, which is a different answer from the one
/// the turn loop gets. A turn resolves its model with
/// [`providers::effective_model`], which prefers `cloud_model` whenever that
/// provider is ready — and on a local setup `cloud_model` is typically a `custom:`
/// id pointing at loopback. So the two halves of the app disagreed about what "the
/// model" meant, and the disagreement was expensive rather than cosmetic.
///
/// Measured on the owner's machine: chat ran `custom:qwen2.5:7b-instruct` (4.7 GB)
/// while every background pass ran `gemma4:12b` (7.6 GB), against ~12.1 GB of free
/// device memory. The two cannot be resident together, so each background pass
/// evicted the model the user was talking to.
///
/// The reload that costs is **not** the point, and an earlier version of this comment
/// said it was, quoting 18–57s. Re-measured quiet on 2026-09-07, a reload is ~2s.
/// What actually costs is the **prefix cache**: the runner keeps the prompt it last
/// processed, so a second turn on the same system prompt and tool schemas skips
/// prefill entirely — 32.15s cold against 0.37s warm, an 87× difference on an 8k
/// prompt. Evicting the model throws that away, and the next turn pays full prefill
/// again. Observed live while measuring: qwen2.5, llama3.2 and gemma4 alternating
/// every few seconds, with turns on an unchanged prompt swinging between 0.43s and
/// 127s depending on whether the cache had survived.
///
/// So `core::modelgate` keeps background work away from the user's turn, and this
/// keeps it on the model that is already loaded — which is the half that stops the
/// cache being thrown away at all rather than merely at a better moment.
///
/// Cloud is still never chosen here — the caller has already decided this workload
/// stays local, and billing someone for a background loop is how always-on
/// assistants become unaffordable. An effective model that is *not* local falls back
/// to `ollama_model`, which is what it was before and the right answer for a user
/// whose chat genuinely runs in the cloud.
///
/// The name is stripped of its `provider:` prefix because it is about to be sent to
/// Ollama's own API, which knows `qwen2.5:7b-instruct` and not `custom:qwen2.5:…`.
/// If the custom endpoint turns out not to be Ollama, the model simply will not be
/// in the installed list and `ollama()` below falls back exactly as it always has.
pub(super) fn local_chat_model(settings: &Settings) -> String {
    let effective = providers::effective_model(settings);
    let local = providers::runs_locally(&effective);
    pick_local_chat_model(&effective, local, &settings.ollama_model)
}

/// The choice itself, split from the two store lookups behind it so it can be
/// tested — the same split, for the same reason, as `providers::resolve_base`
/// against `providers::base_url`.
///
/// `local` is a parameter rather than a call because `runs_locally` on a `custom:`
/// id reads the provider-override store to find the address, and that store is the
/// user's own. A test that let it call through would pass on a machine with a
/// loopback endpoint configured and fail on one without — which is worse than no
/// test, because it is a test that reports the machine it ran on.
pub(super) fn pick_local_chat_model(effective: &str, local: bool, ollama_model: &str) -> String {
    if local {
        providers::model_name(effective).to_string()
    } else {
        ollama_model.to_string()
    }
}

pub async fn complete(
    settings: &Settings,
    messages: &[ChatMessage],
    workload: Workload,
) -> Result<(String, String), String> {
    match route(settings, workload) {
        (Provider::Claude, _) => claude(settings, messages).await,
        (Provider::Ollama, model) => ollama(settings, messages, &model).await,
    }
}

pub(super) async fn ollama(settings: &Settings, messages: &[ChatMessage], wanted: &str) -> Result<(String, String), String> {
    use crate::core::ollama as ol;

    // Discover installed models and resolve a usable one (the configured model
    // may not be pulled — fall back to any installed chat model).
    let installed = ol::list_model_infos(settings)
        .await
        .map_err(|e| format!("Ollama isn't reachable. Start it with `ollama serve`. ({e})"))?;
    if installed.is_empty() {
        return Err("Ollama is running but no models are installed. Pull one, e.g. `ollama pull llama3.2`.".into());
    }
    // Ranked, not positional — Ollama's listing order is not a quality order, and taking
    // the first entry once handed this to a 19.7 GB no-tools model. See resolve_chat_model_info.
    let model = ol::resolve_chat_model_info(&installed, wanted).ok_or_else(|| {
        let names: Vec<&str> = installed.iter().map(|m| m.name.as_str()).collect();
        format!("None of the installed models can chat ({}). Pull a chat model, e.g. `ollama pull llama3.2`.", names.join(", "))
    })?;

    let url = format!("{}/api/chat", settings.ollama_url.trim_end_matches('/'));
    let client = crate::core::egress::client("model");
    let resp = client
        .post(&url)
        .json(&json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": { "num_ctx": settings.num_ctx, "num_predict": settings.max_tokens },
            "keep_alive": crate::core::ollama::KEEP_ALIVE
        }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Ollama at {}. ({e})", settings.ollama_url))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let lower = text.to_lowercase();
        if lower.contains("unknown model architecture") || lower.contains("mllama") {
            return Err(format!("Ollama couldn't load “{model}” — your Ollama version doesn't support this model's architecture. Update Ollama (vision models like llama3.2-vision need Ollama 0.4+), or pick a different model from the model menu."));
        }
        return Err(format!("Ollama error {status}: {}", text.chars().take(200).collect::<String>()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["message"]["content"].as_str().unwrap_or("").trim().to_string();
    if content.is_empty() {
        return Err(format!("Ollama ({model}) returned an empty response."));
    }
    Ok((content, format!("ollama:{model}")))
}
