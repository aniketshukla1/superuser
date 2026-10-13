use super::*;

/// Everything the provider branches share: the resolved model, the tools, the prompt,
/// the tool context and the counters both loops feed.
pub(super) struct Turn<'a> {
    pub(super) app: &'a tauri::AppHandle,
    pub(super) state: &'a AppState,
    pub(super) sink: Option<EventSink>,
    pub(super) exec: crate::core::agent_exec::ToolCtx<'a>,
    pub(super) persona: String,
    pub(super) message: String,
    pub(super) sid: String,
    pub(super) model: String,
    pub(super) api_model: String,
    pub(super) depth: u8,
    pub(super) settings: crate::core::settings::Settings,
    pub(super) prov: Option<&'static crate::core::providers::Provider>,
    pub(super) profile: crate::core::model_profile::ModelProfile,
    pub(super) model_can_think: bool,
    pub(super) shape: crate::core::loopshape::LoopShape,
    pub(super) http: crate::core::egress::Client,
    pub(super) specs: Vec<crate::core::agent::ToolSpec>,
    pub(super) held: Vec<crate::core::agent::ToolSpec>,
    pub(super) skills: crate::core::skills::SkillStore,
    pub(super) mcp: McpBundle,
    pub(super) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(super) working_dir: std::path::PathBuf,
    pub(super) budget: crate::core::context::Budget,
    pub(super) max_steps: usize,
    pub(super) sys_prompt: crate::core::llm::SystemPrompt,
    pub(super) speculator: crate::core::speculation::Speculator,
    pub(super) compaction_watch: Option<crate::core::compaction::Watch>,
    pub(super) tally: Tally,
}

/// What the turn took. HARNESS-PLAN Phase 0: every later phase claims a number — 60–80%
/// off input, step latency from sum to max, TTFT under a second — and none of those claims
/// is checkable against a turn that never said what it cost or how long it took. Kept
/// here so both provider branches feed the same counters rather than growing a second,
/// subtly different pair.
#[derive(Default)]
pub(super) struct Tally {
    pub(super) steps_taken: u32,
    /// Tool time, both ways, summed over the turn's steps — see `StepTiming`.
    pub(super) tool_wall: std::time::Duration,
    pub(super) tool_summed: std::time::Duration,
    pub(super) tools_parallel: u32,
}

/// What a provider branch produced.
pub(super) struct Ran {
    pub(super) answer: String,
    /// None for a branch that reports nothing (the CLI handovers), which is exactly what
    /// "unknown" should look like downstream.
    pub(super) usage: Option<crate::core::sessions::TokenUsage>,
    /// H2: what this turn does with tools, kept so the next turn can see it. Captured from
    /// the live `messages` vec, which is the only place it has ever existed.
    pub(super) wire: Option<crate::core::transcript::TurnWire>,
}

impl Ran {
    pub(super) fn answer(answer: String) -> Self {
        Ran { answer, usage: None, wire: None }
    }
}
