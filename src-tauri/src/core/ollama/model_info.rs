use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<String>,
    pub chat_model: String,
    pub chat_model_ok: bool,
    pub embed_model: String,
    pub embed_model_ok: bool,
    pub error: Option<String>,
}

/// How long a local runner is asked to hold a model in memory after answering.
///
/// Ollama's default is five minutes, so a pause longer than a coffee refill meant
/// the next question paid a model reload before a token was considered.
///
/// **Worth about two seconds, and it is important not to oversell it.** Measured on
/// the owner's M1 Pro, 2026-09-07, three trials with a genuine `ollama stop` between
/// them: reloading `qwen2.5:7b-instruct` cost **2.03–2.13s** against **0.50s** warm.
/// An earlier note here claimed 18–57s; that number was taken on a contended machine
/// and did not survive a quiet one. On Apple Silicon the weights are mmap'd and stay
/// in the page cache, so a reload is far cheaper than the first load of the day.
///
/// For scale, on the same machine and model: prefill runs at ~245 tok/s, so a typical
/// 8k-token step costs ~31s. Holding the model resident saves ~2s of that. This is a
/// real fix and a small one — the prompt is the latency, not the load.
///
/// Thirty minutes rather than `-1` (forever): a model nobody has touched in half an
/// hour should give its memory back, and someone who switches models for good should
/// not have to restart the app to reclaim the old one's RAM.
pub const KEEP_ALIVE: &str = "30m";

/// Ask a local runner to keep this model loaded. A no-op for anything else.
///
/// `keep_alive` is Ollama's own field. It is honoured on the native `/api/*` routes
/// and — measured, not assumed — on the OpenAI-compatible `/v1/chat/completions`
/// shim too, which is what a `custom:` provider pointed at loopback actually talks
/// to. It is gated on [`runs_locally`](crate::core::providers::runs_locally) rather
/// than on the provider *class*, for the reason that function's own comment gives:
/// `custom` is registered `local: false` while being the way most people reach
/// Ollama. The address is the honest test.
///
/// Gating it also bounds the blast radius. A strict cloud endpoint could answer 400
/// to a field it has never heard of, and losing a turn to a latency optimisation
/// would be a poor trade — so a non-local model is never sent one. The send site
/// keeps a retry that drops it anyway, on the same reasoning as `stream_options`.
pub fn stamp_keep_alive(body: &mut Value, model: &str) {
    if crate::core::providers::runs_locally(model) {
        body["keep_alive"] = Value::String(KEEP_ALIVE.to_string());
    }
}

/// "llama3.2:latest" -> "llama3.2"; "qwen2.5:7b-instruct" -> "qwen2.5".
pub fn base_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

/// Looks like an embedding-only model (shouldn't be used for chat).
pub fn is_embed_model(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("embed") || n.contains("bge") || n.contains("minilm") || n.contains("nomic")
}

/// Looks like a multimodal/vision model (can read images), by name.
///
/// Prefer [`supports_vision`] wherever `/api/tags` metadata is in hand. This list is
/// folklore: `gemma4:12b` reads images and matches nothing here, and the consequence
/// is a user who attached an image being switched to a different model — or refused
/// outright by computer use. It survives as the fallback for an Ollama too old to
/// report capabilities, where a name is genuinely all there is.
pub fn is_vision_model(name: &str) -> bool {
    let n = name.to_lowercase();
    ["vision", "llava", "bakllava", "moondream", "minicpm-v", "llama3.2-vision", "qwen2-vl", "qwen2.5-vl", "gemma3", "granite3.2-vision", "pixtral"]
        .iter()
        .any(|k| n.contains(k))
}

/// Is `wanted` installed? Matches exact tag or the base name (so "llama3.2"
/// matches an installed "llama3.2:latest").
pub fn model_available(models: &[String], wanted: &str) -> bool {
    !wanted.is_empty() && models.iter().any(|m| m == wanted || base_name(m) == base_name(wanted))
}

/// Resolve a usable chat model: the wanted one if installed, else the first
/// installed non-embedding model, else None.
///
/// Prefer [`resolve_chat_model_info`] wherever `/api/tags` metadata is available —
/// "first non-embedding" is position-dependent, and Ollama's order is not a quality
/// ranking. See that function for what went wrong with this one.
pub fn resolve_chat_model(models: &[String], wanted: &str) -> Option<String> {
    if let Some(exact) = models.iter().find(|m| *m == wanted || base_name(m) == base_name(wanted)) {
        return Some(exact.clone());
    }
    models.iter().find(|m| !is_embed_model(m)).cloned()
}

/// What `/api/tags` knows about an installed model, beyond its name.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    /// On-disk size in bytes — a good proxy for whether it fits in RAM.
    pub size: u64,
    /// Ollama reports a `tools` capability; the agent loop is tool-driven, so a model
    /// without it can't complete a turn no matter how good it is.
    pub tools: bool,
    /// Reports a `thinking` capability — it can return its reasoning separately from
    /// its answer. Asking a model that can't for `think: true` makes Ollama reject
    /// the request, so this has to be checked rather than assumed.
    pub thinking: bool,
    /// Reports a `vision` capability. Read from the tag rather than guessed from the
    /// name: `gemma4:12b` has vision and says nothing about it in its id, and
    /// [`is_vision_model`]'s keyword list answers `false` for exactly that case.
    pub vision: bool,
    /// The model's real context window, in tokens, as Ollama reports it in
    /// `details.context_length`. `None` on an Ollama old enough not to send it —
    /// which is a different fact from a small window and must not be read as one.
    pub context_length: Option<u32>,
    /// Whether the tag carried a `capabilities` array at all.
    ///
    /// Without this, "this Ollama is too old to say" and "this model has no
    /// capabilities" are the same `false`, and every model on an older Ollama reads
    /// as unable to call tools — which would refuse every turn on those installs.
    pub reported_capabilities: bool,
}

/// Whether `model` can return its reasoning. Unknown models answer `false`: not
/// showing thinking is a missing nicety, while a rejected request is a broken turn.
pub fn supports_thinking(infos: &[ModelInfo], model: &str) -> bool {
    infos.iter().any(|m| m.name == model && m.thinking)
}

/// Whether `model` can read images, preferring what Ollama reported over its name.
///
/// The capability wins when the tag carries one. Where it does not — an Ollama that
/// predates `capabilities` in `/api/tags` — the name heuristic is the only evidence
/// available, and using it is better than answering `false` for `llava`.
pub fn supports_vision(infos: &[ModelInfo], model: &str) -> bool {
    match infos.iter().find(|m| m.name == model) {
        Some(m) if m.reported_capabilities => m.vision,
        _ => is_vision_model(model),
    }
}

/// Resolve a usable chat model, ranking by what the model can actually do.
///
/// The substitution this replaces took `/api/tags`' FIRST non-embedding entry, which is
/// just whatever Ollama listed first. On a real machine that was a 19.7 GB 30B model with
/// no tool support, auto-selected on a 16 GB laptop: every turn swapped, took tens of
/// seconds per step, couldn't call a tool, and periodically crashed llama-server with a
/// 500 — all while the UI displayed the configured model's name, so nothing pointed at the
/// substitution.
///
/// A fallback's job is to RUN, so the order is: the wanted model, then the smallest
/// tool-capable one (fast, fits, can complete a turn), then the smallest of anything left.
/// Deliberate choices the user makes are honored exactly and never ranked.
pub fn resolve_chat_model_info(models: &[ModelInfo], wanted: &str) -> Option<String> {
    if let Some(exact) = models
        .iter()
        .find(|m| m.name == wanted || base_name(&m.name) == base_name(wanted))
    {
        return Some(exact.name.clone());
    }
    let smallest = |usable: &mut dyn Iterator<Item = &ModelInfo>| -> Option<String> {
        usable.min_by_key(|m| m.size).map(|m| m.name.clone())
    };
    smallest(&mut models.iter().filter(|m| !is_embed_model(&m.name) && m.tools))
        .or_else(|| smallest(&mut models.iter().filter(|m| !is_embed_model(&m.name))))
}

/// Parse the model names out of an /api/tags response body.
pub fn parse_models(tags: &Value) -> Vec<String> {
    tags["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect()).unwrap_or_default()
}
