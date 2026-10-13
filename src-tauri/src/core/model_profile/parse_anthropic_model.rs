use super::*;

// ---------------------------------------------------------------------------
// The conservative default
// ---------------------------------------------------------------------------

/// What we can say about a model id nobody recognises.
///
/// Conservative means conservative about *claims*, not about capacity: tools stay on
/// (the loop needs them, and every OpenAI-compatible server that this app can reach
/// has them), while thinking, caching, vision and parallel calls — each of which
/// fails a turn when asserted wrongly — stay off until something says otherwise.
pub fn assumed(model: &str) -> ModelProfile {
    let kind = providers::resolve(model)
        .map(|p| p.kind)
        .unwrap_or(ProviderKind::Ollama);
    ModelProfile {
        id: model.to_string(),
        kind,
        context_window: None,
        max_output: None,
        tools: ToolSupport::Sequential,
        thinking: None,
        vision: false,
        pdf: false,
        // Sending `stream_options.include_usage` to a server that predates it is
        // already handled by a retry-without at the send site, so optimism here costs
        // one retry at worst and buys accounting at best.
        reports_usage: true,
        strict_schemas: false,
        caching: None,
        tier: Tier::Mid,
        source: ProfileSource::Assumed,
    }
}

// ---------------------------------------------------------------------------
// Ollama — from /api/tags, which already carries everything we need
// ---------------------------------------------------------------------------

/// Build a profile from what `/api/tags` reported.
///
/// The plan proposed a second `/api/show` round trip for `context_length`. It isn't
/// needed: Ollama has carried `details.context_length` and `capabilities[]` in the tag
/// listing since 0.30, and the listing is fetched once per turn already.
pub fn from_ollama(model: &str, infos: &[ModelInfo]) -> Option<ModelProfile> {
    let info = infos.iter().find(|m| m.name == model)?;
    Some(ModelProfile {
        id: model.to_string(),
        kind: ProviderKind::Ollama,
        context_window: info.context_length.filter(|w| *w > 0),
        max_output: None,
        tools: if info.tools || !info.reported_capabilities {
            // Ollama advertises `tools`, never "parallel tools". Sequential is the
            // safe read: claiming parallel wrongly means a step whose second call is
            // silently dropped.
            //
            // An Ollama too old to report capabilities gets `Sequential` rather than
            // `None`. Read literally, an absent list says every model is chat-only,
            // and the caller acts on that by refusing the turn — so a missing report
            // must mean "unknown", never "no".
            ToolSupport::Sequential
        } else {
            ToolSupport::None
        },
        thinking: info.thinking.then_some(ThinkingStyle::Flag),
        // Same rule, with the name heuristic as the fallback evidence: see
        // [`ollama::supports_vision`].
        vision: crate::core::ollama::supports_vision(&[info.clone()], model),
        // Ollama takes images, not PDF content blocks.
        pdf: false,
        reports_usage: true,
        strict_schemas: false,
        // Ollama reuses a matching prefix from its own KV cache with nothing sent.
        caching: Some(CacheStyle::Implicit),
        tier: local_tier(info.size),
        // An Ollama that reports no capabilities has told us the model exists and how
        // big its window is, and nothing else. That is a shipped guess wearing a
        // discovery's clothes unless it is labelled.
        source: if info.reported_capabilities {
            ProfileSource::Discovered
        } else {
            ProfileSource::Assumed
        },
    })
}

/// Loop shape for a local model, by what it takes off disk.
///
/// Deliberately tops out at `Mid`: `Frontier` sets the most generous loop shape, and a
/// 70B running on someone's laptop needs the *opposite* of generous. Size is a proxy
/// for "fits in RAM and answers quickly", which is what the loop shape cares about.
pub(super) fn local_tier(size_bytes: u64) -> Tier {
    const GB: u64 = 1_000_000_000;
    if size_bytes < 6 * GB {
        Tier::Small
    } else {
        Tier::Mid
    }
}

// ---------------------------------------------------------------------------
// Anthropic — from GET /v1/models, which answers every question this struct asks
// ---------------------------------------------------------------------------

/// Parse one entry of Anthropic's `GET /v1/models` response.
///
/// The shape is from the live API reference (read 2026-09-02) and is worth reading
/// closely, because it is the authoritative version of tables this repo maintains by
/// hand: `capabilities.thinking.types.{adaptive,enabled}` is what `core::effort`'s
/// `ADAPTIVE` / `REJECTS_BUDGET` lists encode, and `capabilities.effort.*` is its
/// `EFFORT` table. Where a key is present, it wins.
pub fn parse_anthropic_model(entry: &Value) -> Option<ModelProfile> {
    let id = entry["id"].as_str()?.to_string();
    let caps = &entry["capabilities"];
    let supported = |path: &str| caps[path]["supported"].as_bool().unwrap_or(false);

    let thinking = if caps["thinking"]["supported"].as_bool().unwrap_or(false) {
        let types = &caps["thinking"]["types"];
        // Adaptive first: where a model has both, adaptive is the newer contract and
        // the one that does not need a budget guessed for it.
        if types["adaptive"]["supported"].as_bool().unwrap_or(false) {
            Some(ThinkingStyle::Adaptive)
        } else if types["enabled"]["supported"].as_bool().unwrap_or(false) {
            Some(ThinkingStyle::Budget)
        } else {
            None
        }
    } else {
        None
    };

    Some(ModelProfile {
        id: id.clone(),
        kind: ProviderKind::Anthropic,
        // The documented example ships `0` as a placeholder for both. A zero window
        // would floor the input budget and trim away the user's own message, so zero
        // is read as "not reported" — which it is.
        context_window: entry["max_input_tokens"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0),
        max_output: entry["max_tokens"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0),
        tools: ToolSupport::Parallel,
        thinking,
        vision: supported("image_input"),
        pdf: supported("pdf_input"),
        reports_usage: true,
        strict_schemas: supported("structured_outputs"),
        caching: Some(CacheStyle::Explicit),
        tier: cloud_tier(&id),
        source: ProfileSource::Discovered,
    })
}

// ---------------------------------------------------------------------------
// The shipped table — what we know without asking
// ---------------------------------------------------------------------------

/// Model ids whose loop shape is the generous one.
pub(super) const FRONTIER: &[&str] = &[
    "claude-opus", "claude-sonnet", "gpt-5", "gpt-5.5", "gpt-5.6", "grok-4",
    "gemini-2.5-pro", "deepseek-chat", "deepseek-reasoner", "mistral-large",
];

/// A cloud id gets the generous loop shape only if it is one we recognise. Substring
/// matching is right here and wrong for capability: "is this a big model" tolerates a
/// near miss, "does this model reject `budget_tokens`" does not — which is why
/// `core::effort` keeps exact lists instead.
pub(super) fn cloud_tier(model: &str) -> Tier {
    let name = providers::model_name(model).to_lowercase();
    // Checked first: a "mini", "flash" or "haiku" id can also contain a frontier
    // family name, and the smaller variant is the one that decides the loop shape.
    if ["mini", "flash", "haiku", "small", "lightning", "-20b"]
        .iter()
        .any(|k| name.contains(k))
    {
        return Tier::Mid;
    }
    if FRONTIER.iter().any(|k| name.starts_with(k)) {
        Tier::Frontier
    } else {
        Tier::Mid
    }
}
