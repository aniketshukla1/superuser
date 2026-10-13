
/// What one step's tool phase cost, both ways.
///
/// `wall` is how long the user waited; `summed` is how long the calls took added up.
/// Two numbers rather than one because the whole claim of Phase 4 is the gap between
/// them — "step wall-clock ≈ the slowest read, not the sum" cannot be checked against
/// a single figure, which is why HARNESS-BASELINE listed this as a measurement gap
/// before the phase could prove anything.
#[derive(Debug, Clone, Copy, Default)]
pub struct StepTiming {
    pub wall: std::time::Duration,
    pub summed: std::time::Duration,
    pub parallel: usize,
}

/// What one turn took: the numbers HARNESS-PLAN Phase 0 exists to produce.
///
/// Separate from `TokenUsage` because these are facts about the *run*, not about
/// billing, and because `TokenUsage` is persisted per message while this describes
/// the turn as a whole.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TurnMeasure {
    /// Model calls, not tool calls. A turn is several of these; `max_steps` caps it.
    pub steps: u32,
    pub seconds: f64,
    /// `None` means we have no rate for this model id — NOT that it was free. A
    /// local model is `Some(0.0)`; see `core::pricing`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Share of input served from the cache, 0.0–1.0. `None` when there was no input
    /// to speak of, so "the cache did nothing" and "nothing was asked" stay distinct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit_rate: Option<f64>,
    /// Wall-clock spent running tools across every step of the turn.
    pub tool_seconds: f64,
    /// The same calls added up. Equal to `tool_seconds` when nothing overlapped;
    /// larger when something did. The gap is what Phase 4 buys, measured rather than
    /// asserted.
    pub tool_seconds_summed: f64,
    /// How many calls were eligible to run together, across the turn.
    pub tools_parallel: u32,
}

#[derive(serde::Serialize)]
pub struct AgentRunOut {
    pub answer: String,
    /// Tokens across every step of the turn, summed. A turn is several model calls — the
    /// last one's usage is not what the turn cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::core::sessions::TokenUsage>,
    /// What the turn took, beside what it spent. Absent only when nothing ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure: Option<TurnMeasure>,
    pub provider: String,
    pub persona: String,
    pub todos: Vec<crate::core::agent::TodoItem>,
    pub artifacts: Vec<String>,
    /// The charts this turn drew, as replayable tool items. `artifacts` names the
    /// files; these carry the specs, so a reopened conversation shows the chart it
    /// showed the first time instead of a sentence referring to one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub charts: Vec<serde_json::Value>,
    /// This turn's native tool traffic, for the next turn to replay. `None` when the
    /// turn called no tools — its answer already says everything it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<crate::core::transcript::TurnWire>,
    pub working_dir: String,
}

/// Sorted by name, always — for the same reason [`tools_claude`] is, and with more
/// at stake.
///
/// The tools array is the first thing a prompt-cache prefix has to match, so any
/// drift in its order invalidates the whole prompt behind it. That argument was
/// written next door for the Anthropic path and not applied here, where the miss is
/// far more expensive: measured on the owner's M1 Pro, 2026-09-07, an 8k-token
/// prompt costs **32.15s** to prefill cold and **0.37s** when the runner still holds
/// the prefix. A cloud provider absorbs a cache miss in a fraction of a second; a
/// local one charges half a minute for it.
///
/// Capability expansion happens to produce a stable order today. That is not a
/// guarantee, and a cache that silently stops hitting because a capability list was
/// reordered reports nothing — it just gets slow.
pub(in crate::commands) fn tools_ollama(specs: &[crate::core::agent::ToolSpec]) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = specs
        .iter()
        .map(|t| serde_json::json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.schema}}))
        .collect();
    out.sort_by(|a, b| {
        a["function"]["name"].as_str().unwrap_or("").cmp(b["function"]["name"].as_str().unwrap_or(""))
    });
    out
}

/// Sorted by name, always.
///
/// The tools array is the FIRST thing in the cache hierarchy (`tools` → `system` →
/// `messages`), so any drift in its order invalidates everything. The order here comes
/// from capability expansion, which is stable today and is not guaranteed to be — and
/// a cache that silently stops hitting because a capability list was reordered is the
/// kind of regression nothing reports.
pub(in crate::commands) fn tools_claude(specs: &[crate::core::agent::ToolSpec]) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = specs
        .iter()
        .map(|t| serde_json::json!({"name":t.name,"description":t.description,"input_schema":t.schema}))
        .collect();
    out.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    out
}

/// An assistant message assembled from a streamed response.
pub(crate) struct StreamedMessage {
    /// Same shape the non-streaming response produced, so the agent loop is unchanged.
    pub message: serde_json::Value,
    /// The stop flag went up mid-stream; the caller should end the turn.
    pub stopped: bool,
    /// What the provider said the turn cost. None when it said nothing.
    pub usage: Option<crate::core::sessions::TokenUsage>,
    /// Nothing arrived for this long, so the stream was abandoned — see `core::stall`.
    pub stalled: Option<std::time::Duration>,
}

/// Accumulates a streamed reply, one wire line at a time.
///
/// Split out from the HTTP loop so both wire formats can be tested directly — the parsing
/// is where the sharp edges are (fragmented arguments, `[DONE]`, keep-alive blanks), and
/// none of it needs a socket.
#[derive(Default)]
pub(crate) struct StreamAccum {
    pub content: String,
    /// The model's reasoning, when it reports it separately from its answer.
    /// Ollama calls this `thinking`; OpenAI-style reasoning models call it
    /// `reasoning_content`. Kept apart from `content` because it is not the answer —
    /// it goes to the collapsed Thinking block, never into the transcript.
    pub thinking: String,
    pub calls: Vec<serde_json::Value>,
    pub done: bool,
    /// What the provider said the turn cost. None until it says — and some never do.
    pub usage: Option<crate::core::sessions::TokenUsage>,
}

/// One line's worth of stream, split by what it is. Both halves can be empty.
#[derive(Default)]
pub(crate) struct StreamDelta {
    pub text: Option<String>,
    pub thinking: Option<String>,
}
