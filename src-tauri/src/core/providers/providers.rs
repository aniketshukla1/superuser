use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAiCompatible,
    Anthropic,
    Ollama,
    /// Codex, driven through the signed-in `codex` CLI.
    ///
    /// Unlike every other kind here, this one does not serve *our* tool loop — the
    /// Codex CLI runs its own agent, with its own tools and sandbox. Picking it
    /// hands the whole turn over rather than swapping the model behind our loop,
    /// which is why the picker labels it as delegation instead of as a model.
    ///
    /// It exists because it is the honest way to reach a ChatGPT plan: the CLI is
    /// OpenAI's own client, signed in by the user, identifying us as ourselves.
    CodexCli,
    /// Claude Code, driven through the signed-in `claude` CLI — the same handover as
    /// `CodexCli`, for the same reason: it is the honest way to reach a Claude plan.
    /// `core::claude_cli` has the rest.
    ClaudeCli,
}

impl ProviderKind {
    /// Picking this hands the whole turn to another agent rather than swapping the
    /// model behind our loop. Every place that treats Codex specially for that reason
    /// asks this instead, so the two delegated CLIs cannot drift apart.
    pub fn delegates(self) -> bool {
        matches!(self, ProviderKind::CodexCli | ProviderKind::ClaudeCli)
    }
}

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ProviderKind,
    /// OpenAI-compatible base URL (…/v1). Unused for Ollama/Anthropic/Gemini.
    pub base_url: &'static str,
    pub key_hint: &'static str,
    /// Curated, tool-calling-verified models (the model checklist).
    pub models: &'static [&'static str],
    /// Local (no API key needed).
    pub local: bool,
}

pub const PROVIDERS: &[Provider] = &[
    Provider { id: "ollama", name: "Ollama (local)", kind: ProviderKind::Ollama, base_url: "", key_hint: "", models: &[], local: true },
    Provider { id: "anthropic", name: "Anthropic", kind: ProviderKind::Anthropic, base_url: "https://api.anthropic.com", key_hint: "sk-ant-…", models: &["claude-opus-5", "claude-sonnet-5", "claude-opus-4-8", "claude-haiku-4-5"], local: false },
    Provider { id: "openai", name: "OpenAI", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.openai.com/v1", key_hint: "sk-…", models: &["gpt-5.5", "gpt-5", "gpt-5-mini"], local: false },
    // Gemini ships an OpenAI-compatible surface, so it uses the same tool-calling
    // path as the rest rather than needing its own generateContent client.
    Provider { id: "gemini", name: "Google Gemini", kind: ProviderKind::OpenAiCompatible, base_url: "https://generativelanguage.googleapis.com/v1beta/openai", key_hint: "AIza…", models: &["gemini-2.5-pro", "gemini-2.5-flash"], local: false },
    Provider { id: "deepseek", name: "DeepSeek", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.deepseek.com/v1", key_hint: "sk-…", models: &["deepseek-chat", "deepseek-reasoner"], local: false },
    Provider { id: "xai", name: "xAI (Grok)", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.x.ai/v1", key_hint: "xai-…", models: &["grok-4", "grok-3"], local: false },
    // gpt-oss first: it is the tool-calling model on Groq's current production list,
    // and Groq's draw for this app is that a long agent loop finishes in a fraction
    // of the wall-clock time. `llama-3.3-70b-versatile` is kept because keys that
    // already select it should not break, but it is no longer the one to reach for.
    Provider { id: "groq", name: "Groq", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.groq.com/openai/v1", key_hint: "gsk_…", models: &["openai/gpt-oss-120b", "openai/gpt-oss-20b", "llama-3.3-70b-versatile"], local: false },
    Provider { id: "mistral", name: "Mistral", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.mistral.ai/v1", key_hint: "…", models: &["mistral-large-latest", "mistral-small-latest"], local: false },
    Provider { id: "together", name: "Together", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.together.xyz/v1", key_hint: "…", models: &["Qwen/Qwen2.5-72B-Instruct-Turbo", "deepseek-ai/DeepSeek-V3"], local: false },
    Provider { id: "fireworks", name: "Fireworks", kind: ProviderKind::OpenAiCompatible, base_url: "https://api.fireworks.ai/inference/v1", key_hint: "fw_…", models: &["accounts/fireworks/models/deepseek-v3"], local: false },
    // Anything else that speaks the OpenAI wire format: a gateway, a self-hosted
    // vLLM or LM Studio, a proxy on loopback. It ships with no address because the
    // address *is* the configuration — `base_url` reads it from the override store,
    // and there are no curated models because we can't know what it serves. Models
    // are named `custom:<model>`, which `resolve` already routes by prefix.
    Provider { id: "custom", name: "Custom (OpenAI-compatible)", kind: ProviderKind::OpenAiCompatible, base_url: "", key_hint: "optional", models: &[], local: false },
    // OpenCode Zen: one key, ~64 curated models across Anthropic, OpenAI, Google, xAI and
    // the open-weight houses. Registered as its own provider rather than as a `custom`
    // base-URL because `custom` is the local escape hatch — it is usually pointed at Ollama
    // or an MLX server on loopback, and a user should not have to give that up to reach a
    // cloud model.
    //
    // Only the OpenAI-compatible surface is used (`/zen/v1/chat/completions`). Zen also
    // exposes Anthropic- and Google-shaped endpoints for their respective families, but one
    // wire format that serves every model is worth more here than per-family fidelity we
    // would then have to keep matched.
    //
    // The curated list is the free tier plus the two flagships. The free ones are what a key
    // with no payment method can actually run — everything else answers "No payment method",
    // and a picker full of ids that every turn rejects is worse than a short one that works.
    Provider {
        id: "opencode",
        name: "OpenCode Zen",
        kind: ProviderKind::OpenAiCompatible,
        base_url: "https://opencode.ai/zen/v1",
        key_hint: "from `opencode auth login`",
        // The free tier, then the two flagships. Every free id here was checked against a
        // tool-calling request and answered with a real `tool_calls` — which is the bar that
        // matters, because this app's agent loop is tool calls and a model that only chats
        // cannot run a turn. `mimo-v2.5-free` and `muse-spark-1.2-contributor-free` are
        // omitted: they answered "Rate limit exceeded" and "Internal server error" on the
        // same check, and an id in the picker that fails at turn time is worse than absent.
        //
        // The paid ids are last because a key without a payment method gets "No payment
        // method" from all of them — they are here so the list does not need editing the day
        // billing is added, not because they work today.
        models: &[
            "big-pickle",
            "x-preview-f-free",
            "laguna-s-2.1-free",
            "nemotron-3-ultra-free",
            "nemotron-3.5-lightning-free",
            "hy3-free",
            "claude-sonnet-5",
            "gpt-5.6-sol",
        ],
        local: false,
    },
    // No key and no address: the credential is the `codex login` the user already
    // did, and it stays in the CLI's keeping rather than ours. The models are the
    // ones a ChatGPT plan actually serves — the CLI knows others, but they answer
    // "not supported when using Codex with a ChatGPT account", so offering them
    // would put ids in the picker that every turn rejects.
    Provider { id: "codex", name: "Codex (ChatGPT plan)", kind: ProviderKind::CodexCli, base_url: "", key_hint: "", models: &["gpt-5.6-sol", "gpt-5.6-luna", "gpt-5.6-terra"], local: false },
    // The same arrangement for Claude: the credential is the user's own `claude` sign-in.
    // Aliases rather than dated ids, so the CLI resolves each to whatever its current
    // version of that model is — a plan's lineup moves, and a pinned id in this list
    // would start failing the day it did.
    Provider { id: "claude-code", name: "Claude Code", kind: ProviderKind::ClaudeCli, base_url: "", key_hint: "", models: &["opus", "sonnet", "haiku"], local: false },
];

/// The API key for a provider — ours if one was saved, otherwise the one the vendor's own
/// CLI already holds.
///
/// For OpenCode Zen that is `~/.local/share/opencode/auth.json`, written by
/// `opencode auth login`. Reading it means someone who has already signed in to the CLI
/// does not have to find the key and paste it in again, which is the same courtesy the
/// Codex provider extends by using the CLI's session rather than asking for one.
///
/// Ours wins when both exist: a key typed into this app is a deliberate choice, and a CLI
/// login that happens to be on the machine should never silently override it.
pub fn key_for(id: &str) -> Option<String> {
    if let Some(k) = crate::core::secrets::get_provider_key(id) {
        return Some(k);
    }
    if id == "opencode" {
        return opencode_cli_key();
    }
    None
}

/// Read the OpenCode CLI's stored key. Returns None for every failure — a missing file, a
/// changed shape, a login that has not happened — because "no key" is the right answer to
/// all of them and none is worth an error the caller has to handle.
pub(super) fn opencode_cli_key() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::Path::new(&home).join(".local/share/opencode/auth.json");
    let text = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let k = v["opencode"]["key"].as_str()?.trim();
    if k.is_empty() { None } else { Some(k.to_string()) }
}

/// Whether the saved cloud model can actually run — a question about *its own*
/// provider.
///
/// The thing this replaces asked `secrets::has_cloud_key()`, the Anthropic key, for
/// every provider alike. So choosing Groq, Codex or a custom endpoint saved the id
/// and then read the local one back, and the app quietly ran a model the user had
/// not picked; while an Anthropic key made an unconfigured cloud model look ready.
/// One key answering for every vendor was wrong in both directions.
pub fn cloud_model_ready(model: &str) -> bool {
    match resolve(model) {
        Some(p) => list_info().iter().any(|i| i.id == p.id && i.ready),
        None => false,
    }
}
