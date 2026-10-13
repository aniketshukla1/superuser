use super::*;

/// Whether the model can call tools, and whether it can call several at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSupport {
    /// Chat only. This app's turn *is* tool calls, so such a model cannot complete one.
    None,
    /// One tool call per step.
    Sequential,
    /// Several tool calls in a single assistant message.
    Parallel,
}

/// How this model is asked to show its working. The same slider, four vocabularies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingStyle {
    /// Anthropic `thinking: {type: "adaptive"}` — 4.7 and later.
    Adaptive,
    /// Anthropic `thinking: {type: "enabled", budget_tokens}` — 4.5 and earlier.
    Budget,
    /// OpenAI-compatible `reasoning_effort: "low" | "medium" | "high"`.
    EffortName,
    /// Ollama `think: true`. A flag, with no dial behind it.
    Flag,
}

/// Whether a cache prefix is something we mark, or something the server finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStyle {
    /// Breakpoints the caller places: Anthropic `cache_control`. Phase 3 shapes the
    /// prompt for exactly this.
    Explicit,
    /// The server reuses a matching prefix by itself — Ollama's KV cache. Nothing to
    /// send, but a stable prefix is still what makes it work.
    Implicit,
}

/// Loop shape. HARNESS-PLAN Phase 5 reads this; nothing does yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Small,
    Mid,
    Frontier,
}

/// How we came to believe this profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSource {
    /// The server told us, this run.
    Discovered,
    /// Our shipped table, for a model id we ship support for.
    Declared,
    /// A handshake request measured it.
    Probed,
    /// Nobody knows. The conservative default, and the UI should say so.
    Assumed,
}

impl ProfileSource {
    /// Whether this profile rests on evidence rather than on a shipped guess.
    pub fn is_evidence(self) -> bool {
        matches!(self, ProfileSource::Discovered | ProfileSource::Probed)
    }
}

/// One model's answer to every question the loop asks.
///
/// `context_window` and `max_output` are `Option` on purpose: **unknown is not small**.
/// A `None` window means "use the configured budget", which is today's behaviour; a
/// `Some(0)` would silently trim the conversation to nothing, so the parsers below
/// reject zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelProfile {
    pub id: String,
    pub kind: ProviderKind,
    pub context_window: Option<u32>,
    pub max_output: Option<u32>,
    pub tools: ToolSupport,
    pub thinking: Option<ThinkingStyle>,
    pub vision: bool,
    /// Accepts PDF content blocks natively. Distinct from `vision`: a model can read
    /// images and still not take a PDF.
    pub pdf: bool,
    /// Reports token usage. Without it, no cost accounting is possible for this model
    /// and `/v1/stats` has a hole rather than a zero.
    pub reports_usage: bool,
    /// Accepts strict tool schemas (`additionalProperties: false` + strict mode).
    /// HARNESS-PLAN §2.17 is gated on this.
    pub strict_schemas: bool,
    pub caching: Option<CacheStyle>,
    pub tier: Tier,
    pub source: ProfileSource,
}

impl ModelProfile {
    /// Per-token rates, from the one table that has them.
    pub fn price(&self) -> Option<crate::core::pricing::Rates> {
        crate::core::pricing::rates_for(&self.id)
    }

    /// Whether this model is served by something on this machine.
    ///
    /// `kind` answers a different question — which *wire format* to speak — and the
    /// two come apart exactly where it matters: an OpenAI-compatible server on
    /// loopback is a cloud class and a local runner. `id` is the full model id here
    /// (`assumed` and every parser store it unstripped), which is what
    /// [`providers::runs_locally`] needs to check the address behind the prefix.
    pub fn runs_locally(&self) -> bool {
        providers::runs_locally(&self.id)
    }

    /// The largest prompt worth assembling for this model, given the user's settings.
    ///
    /// The model's window is a **ceiling**, never a target. `settings.num_ctx` is a
    /// memory setting — on Ollama it decides how much KV cache is allocated at load —
    /// so a model with a 262k window on a 16 GB laptop must still be asked for what
    /// the user allowed. What the window adds is the other direction: it stops us
    /// asking a 32k model to hold 128k because a setting says so.
    pub fn input_budget(&self, settings: &Settings) -> usize {
        let configured = settings.agent_max_context_tokens as usize;
        let reply = settings.max_tokens as usize;
        let ceiling = match self.context_window {
            // The window covers input *and* output on every backend that enforces it,
            // so the reply has to come out of it before this is an input budget.
            Some(w) => (w as usize).saturating_sub(reply),
            None => usize::MAX,
        };
        // The local allocation ceiling, keyed on the ADDRESS rather than on the
        // provider class.
        //
        // This read `self.kind == ProviderKind::Ollama`, which is the same mistake
        // `runs_locally` documents at length: `custom:` is registered as a cloud class
        // while being how most people actually reach Ollama. So the one configuration
        // that most needed the ceiling was the one that skipped it. Measured on the
        // owner's machine, 2026-09-05: `custom:qwen2.5:7b-instruct` has no declared
        // window, so `ceiling` was `MAX` and `local_ceiling` was `MAX`, leaving
        // `agent_max_context_tokens` (48,000) to win outright against a runner Ollama
        // had loaded with a 32,768 context. Ollama does not refuse an overflow, it
        // silently drops the front of the prompt — so the model lost its own system
        // instructions, and the app paid prefill for up to 48k of tokens at ~150 tok/s
        // to do it. That is minutes of latency spent actively making the turn worse.
        let local_ceiling = if self.runs_locally() {
            (self.ollama_num_ctx(settings) as usize).saturating_sub(reply)
        } else {
            usize::MAX
        };
        // Floor it: a pathological setting must not trim away the user's own message.
        configured.min(ceiling).min(local_ceiling).max(2_000)
    }

    /// What to send Ollama as `options.num_ctx`.
    ///
    /// The user's setting, clamped to what the model can actually hold. Asking for
    /// more than the window is not free — it is what makes Ollama allocate a KV cache
    /// the architecture cannot use, and on some builds it is refused outright.
    pub fn ollama_num_ctx(&self, settings: &Settings) -> u32 {
        match self.context_window {
            Some(w) => settings.num_ctx.min(w),
            None => settings.num_ctx,
        }
    }

    /// Whether asking this model to show its working is safe.
    ///
    /// Ollama rejects `think: true` outright for a model without the capability — the
    /// turn fails rather than merely lacking a nicety — which is why this is a
    /// capability read and never an assumption.
    pub fn can_think(&self) -> bool {
        self.thinking.is_some()
    }

    /// Can this model complete a turn at all? The loop is tool calls; a chat-only
    /// model cannot run one, and finding that out at turn time wastes the user's wait.
    pub fn can_run_a_turn(&self) -> bool {
        self.tools != ToolSupport::None
    }
}

/// A context window as a picker would say it: `262144` → `"262k"`.
///
/// Rounded down, never up: a label that overstates what a model holds is the same
/// mistake as a setting that does, one screen earlier.
pub fn human_tokens(tokens: u32) -> String {
    match tokens {
        t if t >= 1_000_000 => format!("{}M", t / 1_000_000),
        t if t >= 1_000 => format!("{}k", t / 1_000),
        t => t.to_string(),
    }
}
