use super::*;

/// Map a model id (`provider:model` or bare) to its provider. A bare id is matched
/// by well-known prefixes; anything unrecognized is treated as a local Ollama model
/// (returns None).
pub fn resolve(model_id: &str) -> Option<&'static Provider> {
    if let Some((prefix, _)) = model_id.split_once(':') {
        return find(prefix);
    }
    let m = model_id.to_lowercase();
    let hit = |pat: &str, id: &str| -> Option<&'static Provider> {
        if m.starts_with(pat) { find(id) } else { None }
    };
    hit("claude", "anthropic")
        .or_else(|| hit("gpt", "openai"))
        .or_else(|| hit("o1", "openai"))
        .or_else(|| hit("o3", "openai"))
        .or_else(|| hit("gemini", "gemini"))
        .or_else(|| hit("deepseek", "deepseek"))
        .or_else(|| hit("grok", "xai"))
        .or_else(|| hit("mistral", "mistral"))
}

/// The bare model name to send on the wire (strips a `provider:` prefix).
/// The provider's own name for a model, stripping a `provider:` prefix if there is one.
///
/// Only strips when the prefix is a REAL provider. Ollama tags models `name:tag`, which
/// looks identical to the `provider:model` convention — stripping unconditionally turned
/// `qwen2.5:7b-instruct` into `7b-instruct`, which is installed nowhere, so picking any
/// tagged local model silently fell back to a different one. Selecting a specific local
/// model was impossible, and nothing said so.
pub fn model_name(model_id: &str) -> &str {
    match model_id.split_once(':') {
        Some((prefix, rest)) if find(prefix).is_some() => rest,
        _ => model_id,
    }
}

/// Provider status for the UI (which have a key, their curated models).
#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub local: bool,
    pub has_key: bool,
    pub key_hint: String,
    pub models: Vec<String>,
    /// The endpoint this provider will actually call, override applied.
    pub base_url: String,
    /// Usable right now. Not the same as `has_key`: a provider pointed at loopback
    /// needs no key, and `custom` needs an address before a key means anything.
    pub ready: bool,
}

pub fn list_info() -> Vec<ProviderInfo> {
    PROVIDERS
        .iter()
        .map(|p| {
            let has_key = if p.local {
                true
            } else if p.kind == ProviderKind::CodexCli {
                // The "key" is a login held by the CLI. Nothing to paste, so the
                // setup form has no field and readiness is a fact about the machine.
                codex_ready()
            } else if p.kind == ProviderKind::ClaudeCli {
                crate::core::claude_cli::ready()
            } else if p.id == "anthropic" {
                crate::core::secrets::has_cloud_key()
            } else {
                // `key_for`, not `has_provider_key`: a provider whose key comes from the
                // vendor's own CLI is configured, and readiness must agree with what the
                // request path will actually use. They disagreed, and the effect was that
                // picking OpenCode Zen saved the id, reported "not ready", and quietly ran
                // the local model instead — the same class of bug the doc comment below
                // this function already describes.
                key_for(p.id).is_some()
            };
            let base = base_url(p.id);
            ProviderInfo {
                id: p.id.into(),
                name: p.name.into(),
                kind: p.kind,
                local: p.local,
                has_key,
                key_hint: p.key_hint.into(),
                models: models_for(p.id),
                ready: p.local
                    || p.kind.delegates() && has_key
                    || (!base.is_empty() && (has_key || is_loopback(&base))),
                base_url: base,
            }
        })
        .collect()
}
