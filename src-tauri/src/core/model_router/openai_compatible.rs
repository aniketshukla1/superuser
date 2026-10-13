use super::*;

/// Best-effort image type from base64 magic bytes (Claude needs a media_type).
pub(super) fn sniff_media_type(b64: &str) -> &'static str {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let prefix_len = (b64.len().min(16) / 4) * 4;
    let head = STANDARD.decode(&b64[..prefix_len]).unwrap_or_default();
    match head.as_slice() {
        [0xFF, 0xD8, ..] => "image/jpeg",
        [0x47, 0x49, 0x46, ..] => "image/gif",                       // GIF
        [0x52, 0x49, 0x46, 0x46, ..] => "image/webp",                // RIFF (webp)
        _ => "image/png",
    }
}

/// Complete on ONE named model, through that model's own provider.
///
/// `complete` routes by *workload* and knows two destinations, Anthropic and Ollama.
/// That was the whole world when it was written, and it is why a side call on a
/// ChatGPT-plan or Groq model went to `api.anthropic.com` carrying a model id
/// Anthropic has never heard of — the id was honoured, the vendor was not.
///
/// `providers::cloud_model_ready` exists for the readiness half of exactly this bug
/// (its own doc says so: one key answering for every vendor). This is the calling
/// half. Every kind in the registry is reachable here, so a side call runs on the
/// model the user actually picked or fails saying which provider refused it.
///
/// Codex is delegation rather than routing — the CLI runs its own agent — so it gets
/// a flattened prompt and an empty scratch directory to work in, never the vault or
/// the user's repo.
pub async fn complete_on(
    settings: &Settings,
    messages: &[ChatMessage],
    model_id: &str,
) -> Result<(String, String), String> {
    use crate::core::providers::{self, ProviderKind};

    let Some(p) = providers::resolve(model_id) else {
        // Unclaimed ids are Ollama tags — `gemma4:12b` looks like `provider:model`
        // and is not one. Same rule `runs_locally` documents.
        return ollama(settings, messages, model_id).await;
    };
    let api_model = providers::model_name(model_id).to_string();
    match p.kind {
        ProviderKind::Ollama => ollama(settings, messages, &api_model).await,
        ProviderKind::Anthropic => {
            // `claude` reads the model off the settings, so the pick has to land
            // there — otherwise the saved cloud model answers for the chosen one.
            let mut s = settings.clone();
            s.cloud_model = api_model;
            claude(&s, messages).await
        }
        ProviderKind::OpenAiCompatible => openai_compatible(settings, messages, p, &api_model).await,
        ProviderKind::CodexCli => codex(settings, messages, &api_model).await,
        ProviderKind::ClaudeCli => claude_code(messages, &api_model).await,
    }
}

/// A plain, non-streaming `/chat/completions` — OpenAI, Groq, Gemini's compatible
/// endpoint, or anything behind a custom base URL.
///
/// No tools and no streaming on purpose: a side call is one question and one answer,
/// and nobody is watching it arrive.
pub(super) async fn openai_compatible(
    settings: &Settings,
    messages: &[ChatMessage],
    p: &'static crate::core::providers::Provider,
    api_model: &str,
) -> Result<(String, String), String> {
    let base = crate::core::providers::base_url(p.id);
    let (url, bearer) =
        crate::core::providers::endpoint(&base, crate::core::providers::key_for(p.id), p.name)?;

    let turns: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();
    let mut req = crate::core::egress::client("model")
        .post(&url)
        .json(&json!({ "model": api_model, "stream": false, "messages": turns }));
    if let Some(k) = bearer {
        req = req.bearer_auth(k);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Could not reach {} at {url}. ({e})", p.name))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "{} error {status}: {}",
            p.name,
            text.chars().take(200).collect::<String>()
        ));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string();
    if content.is_empty() {
        return Err(format!("{} ({api_model}) returned an empty response.", p.name));
    }
    Ok((content, format!("{}:{api_model}", p.id)))
}

/// Hand a side call to the signed-in Codex CLI.
///
/// The prompt is flattened the way `run_agent` flattens a Codex turn, and the
/// working directory is a fresh empty scratch dir: Codex brings its own tools and
/// would otherwise be pointed at whatever happened to be the process's cwd. A
/// question about the user's memory needs no files — everything it may use is in
/// the prompt.
pub(super) async fn codex(
    settings: &Settings,
    messages: &[ChatMessage],
    api_model: &str,
) -> Result<(String, String), String> {
    let _ = settings;
    if !crate::core::providers::codex_ready() {
        return Err("The Codex CLI isn't signed in on this machine. Run `codex login`, or pick another model.".into());
    }
    let system: String =
        messages.iter().filter(|m| m.role == "system").map(|m| m.content.clone()).collect::<Vec<_>>().join("\n");
    let last_user =
        messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
    let prompt = crate::core::codex_cli::flatten(&system, &[], &last_user);

    let dir = std::env::temp_dir().join(format!("superuser-codex-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not make a scratch directory for Codex: {e}"))?;
    // Nothing to stream: a side call has no panel to paint into.
    let text = crate::core::codex_cli::run(api_model, &prompt, &dir, |_| {}).await?;
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(format!("Codex ({api_model}) returned an empty response."));
    }
    Ok((text, format!("codex:{api_model}")))
}

/// Hand a side call to the signed-in Claude Code CLI — `codex` above, for Claude.
pub(super) async fn claude_code(messages: &[ChatMessage], api_model: &str) -> Result<(String, String), String> {
    if !crate::core::claude_cli::ready() {
        return Err("Claude Code isn't signed in on this machine. Run `claude` and sign in, or pick another model.".into());
    }
    let system: String =
        messages.iter().filter(|m| m.role == "system").map(|m| m.content.clone()).collect::<Vec<_>>().join("\n");
    let last_user =
        messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
    let prompt = crate::core::claude_cli::flatten(&system, &[], &last_user);

    // An empty scratch dir, for the reason `codex` uses one: the CLI's tools would
    // otherwise read whatever the process's cwd happens to be.
    let dir = std::env::temp_dir().join(format!("superuser-claude-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not make a scratch directory for Claude Code: {e}"))?;
    let text = crate::core::claude_cli::run(api_model, &prompt, &dir, |_| {}).await?;
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(format!("Claude Code ({api_model}) returned an empty response."));
    }
    Ok((text, format!("claude-code:{api_model}")))
}
