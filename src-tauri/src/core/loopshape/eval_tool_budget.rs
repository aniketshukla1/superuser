use super::*;

/// The knobs a turn sets from the model in front of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopShape {
    /// Tokens of tool schema this model is sent up front. `None` is no limit.
    pub tool_budget: Option<usize>,
    /// Tool-call steps before the model must answer.
    pub max_steps: usize,
    /// Whether the skills catalogue goes in the prompt.
    ///
    /// It is a menu of twenty-one one-line descriptions, and a model that picks the
    /// wrong one spends a step loading it. The same argument as the tool budget, for
    /// the same reason.
    pub skills_in_prompt: bool,
    /// Whether several tools may run at once.
    pub parallel_tools: bool,
}

/// Room for the resident set and the mid-cost capabilities on top of it — knowledge,
/// recall, connectors, subagent — but not the whole 6.5k catalogue.
///
/// **Applied to no tier as of 2026-09-05.** Small never took it (narrowing measured
/// worse); Mid stopped taking it when the deferral was measured to cost a ~30s
/// `find_tools` round trip on a local model. Kept, with `split_tools`, because the
/// machinery is right and only the evidence was missing — see the two tier arms in
/// [`LoopShape::for_model`] for what each was measured to do.
#[allow(dead_code)]
pub(super) const MID_BUDGET: usize = 3_000;

/// The estimator `core::context` uses. Good enough to size a budget, and the same
/// arithmetic the Phase 6 measurement is quoted in, so the two are comparable.
pub(super) fn schema_tokens(t: &ToolSpec) -> usize {
    json!({ "name": t.name, "description": t.description, "input_schema": t.schema })
        .to_string()
        .len()
        / 4
}

/// The loop as it was before any of this: every model gets the whole catalogue, the
/// full step budget, the skills menu, and reads run in parallel on risk class alone.
///
/// This exists so the scoreboard can measure what tier shaping is worth, and it is the
/// only honest way to get that number. The suite is younger than the code it grades —
/// `harness_eval` does not exist at the commit before Phase 5, and neither does the
/// endpoint support the driver needs — so "check out the old commit and run it" cannot
/// be done without backporting the measuring apparatus, which would change the thing
/// being measured. Toggling one function on one binary varies one thing instead of
/// three commits' worth.
///
/// Set `HARNESS_EVAL_FULL_SHAPE=1`. Read once per turn, off by default, and it can only
/// ever *widen* the loop — a stray env var cannot narrow anyone's tools.
pub(super) fn shaping_disabled() -> bool {
    std::env::var("HARNESS_EVAL_FULL_SHAPE").is_ok_and(|v| v == "1")
}

/// A tool budget imposed for the length of a sweep, in schema tokens.
///
/// HARNESS-RESEARCH-2026 §4 asks whether selecting tools *by relevance to the turn*
/// beats the fixed priority order that measured worse. `SUPERUSER_TOOL_RETRIEVAL`
/// exists to answer it and, as shipped, cannot: every tier below carries
/// `tool_budget: None` — each for a reason its own arm documents — and both
/// `split_tools` and `split_tools_retrieved` hand back the entire catalogue the
/// moment the budget is absent.
///
/// So the ablation had two arms that were the same arm. Run in that state it would
/// have produced a table reading "relevance selection makes no difference on any
/// model", which is not a null result — it is a false negative about a code path
/// that never executed. The flag is what gives the comparison something to vary.
///
/// Read once per turn, absent by default, and it can only ever **narrow**: the exact
/// mirror of `HARNESS_EVAL_FULL_SHAPE`, which can only widen. Neither can be set by
/// anything but the operator's own environment, and a stray value cannot widen a
/// persona's capabilities or a subagent's allowlist.
pub(super) fn eval_tool_budget() -> Option<usize> {
    std::env::var("HARNESS_EVAL_TOOL_BUDGET").ok()?.parse().ok().filter(|n| *n > 0)
}
