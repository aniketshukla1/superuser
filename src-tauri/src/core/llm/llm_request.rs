use super::*;

/// One call to a model.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub max_tokens: u32,
    /// A string, or an array of system blocks. Passed through as given, because Phase 3
    /// turns this into blocks with cache breakpoints and this is where that lands.
    pub system: Value,
    pub messages: Vec<Value>,
    /// Empty means "no tools on this request", not "the default set".
    pub tools: Vec<Value>,
    /// The effort LEVEL name (`core::effort::LEVELS`), not an API value — the mapping
    /// is per-model and belongs with the table that knows it.
    pub effort: Option<String>,
    /// An `anthropic-beta` header, for the one caller that needs computer use.
    pub beta: Option<String>,
}

impl LlmRequest {
    pub fn new(model: impl Into<String>, max_tokens: u32) -> Self {
        LlmRequest {
            model: model.into(),
            max_tokens,
            system: Value::Null,
            messages: Vec::new(),
            tools: Vec::new(),
            effort: None,
            beta: None,
        }
    }
    pub fn system(mut self, system: impl Into<Value>) -> Self {
        self.system = system.into();
        self
    }
    pub fn messages(mut self, messages: Vec<Value>) -> Self {
        self.messages = messages;
        self
    }
    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = tools;
        self
    }
    pub fn effort(mut self, level: impl Into<String>) -> Self {
        self.effort = Some(level.into());
        self
    }
    pub fn beta(mut self, header: impl Into<String>) -> Self {
        self.beta = Some(header.into());
        self
    }

    /// The wire body. Split out so the shape can be tested without a socket — which is
    /// the whole reason §2.1 went unnoticed: nothing could look at what we were sending.
    pub fn body(&self) -> Value {
        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": self.messages,
        });
        if !self.system.is_null() {
            body["system"] = self.system.clone();
        }
        if !self.tools.is_empty() {
            body["tools"] = json!(self.deferred_tools());
        }
        if let Some(level) = &self.effort {
            let (thinking, output_config) =
                crate::core::effort::request_fields(&self.model, level, self.max_tokens);
            if let Some(t) = thinking {
                body["thinking"] = t;
            }
            if let Some(o) = output_config {
                body["output_config"] = o;
            }
        }
        body
    }
}

/// A system prompt in two halves, so a cache breakpoint can sit between them.
///
/// HARNESS-PLAN §2.8. Caching is a prefix match: one byte changes and everything after
/// it is invalidated. The app's system string was assembled in near-perfect reverse
/// order — the opened documents, the SuperContext brief (derived from the current
/// turn's message) and a minute-resolution clock all sat AHEAD of the persona, the
/// gates, the skills, AGENTS.md and the tool descriptions. Marking that up with
/// `cache_control` unchanged would have paid the 1.25× write premium on every request
/// and read nothing back.
#[derive(Debug, Clone, Default)]
pub struct SystemPrompt {
    /// Session-stable: persona, core memory, gates, skills, AGENTS.md.
    pub stable: String,
    /// Per-turn: opened documents, the brief, the clock, the dropped-turn summary.
    pub volatile: String,
}

impl SystemPrompt {
    /// The two halves as one string, for providers with no cache to shape for.
    pub fn joined(&self) -> String {
        format!("{}{}", self.stable, self.volatile)
    }

    /// System blocks with the breakpoint after the stable half.
    ///
    /// The volatile half follows it in the same array: still sent, still read, simply
    /// not part of the cached prefix. A short prefix is not an error — the API just
    /// does not cache it, and `cache_read_input_tokens` says so.
    pub fn blocks(&self) -> Value {
        let mut out = vec![json!({
            "type": "text",
            "text": self.stable,
            "cache_control": { "type": "ephemeral" },
        })];
        if !self.volatile.trim().is_empty() {
            out.push(json!({ "type": "text", "text": self.volatile }));
        }
        Value::Array(out)
    }
}

/// Models that accept the tool search tool. From the API docs
/// (platform.claude.com/docs/en/agents-and-tools/tool-use/tool-search-tool, read
/// 2026-09-02), not inferred from the id.
///
/// Note what is NOT here: `claude-sonnet-5`. The support table lists Sonnet 4.5 and
/// Sonnet 4.6 and skips it, which is the kind of gap that version arithmetic on a
/// string would paper over — and the failure would be a 400 on every turn for anyone
/// who picked that model.
pub(super) const TOOL_SEARCH: &[&str] = &[
    "claude-fable-5-1", "claude-mythos-5-1", "claude-fable-5", "claude-mythos-5",
    "claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6",
    "claude-sonnet-4-6", "claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5",
];

/// Below this many tools, searching for one costs more than carrying them all — the
/// docs put the crossover at about ten.
pub(super) const TOOL_SEARCH_MIN: usize = 10;

pub fn supports_tool_search(model: &str) -> bool {
    let bare = crate::core::providers::model_name(model).to_ascii_lowercase();
    TOOL_SEARCH.iter().any(|p| bare.starts_with(p))
}

impl LlmRequest {
    /// The `tools` array as it goes on the wire: the search tool first, then every
    /// definition, with the long tail marked `defer_loading`.
    ///
    /// `defer_loading` controls what enters the CONTEXT WINDOW, not what is sent — the
    /// full definitions still travel on every request so the API can run the search and
    /// expand a `tool_reference`. So this does not shrink the upload; it shrinks the
    /// thing that is billed and that the model has to read past, which is the one that
    /// costs money and attention.
    ///
    /// The search tool is never deferred (the API rejects a request where everything
    /// is), and neither is anything in `agent::RESIDENT`.
    pub(super) fn deferred_tools(&self) -> Vec<Value> {
        if !supports_tool_search(&self.model) || self.tools.len() < TOOL_SEARCH_MIN {
            return self.tools.clone();
        }
        let mut out = Vec::with_capacity(self.tools.len() + 1);
        out.push(json!({ "type": "tool_search_tool_regex_20251119", "name": "tool_search_tool_regex" }));
        for t in &self.tools {
            let mut t = t.clone();
            let name = t["name"].as_str().unwrap_or("");
            if !crate::core::agent::is_resident(name) {
                t["defer_loading"] = json!(true);
            }
            out.push(t);
        }
        out
    }
}

/// What came back. The content blocks are handed over raw: this module has no opinion
/// about what a turn does with them.
#[derive(Debug, Clone, Default)]
pub struct LlmReply {
    pub content: Vec<Value>,
    pub stop_reason: String,
    pub usage: Option<TokenUsage>,
}
