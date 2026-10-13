use super::*;

/// The history this turn sends, and what it keeps an eye on because of what it cut.
pub(super) struct Fitted {
    pub(super) budget: crate::core::context::Budget,
    pub(super) history_msgs: Vec<serde_json::Value>,
    pub(super) speculator: crate::core::speculation::Speculator,
    pub(super) compaction_watch: Option<crate::core::compaction::Watch>,
    pub(super) max_steps: usize,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn fit_context(
    app: &tauri::AppHandle,
    sink: &Option<EventSink>,
    state: &AppState,
    settings: &crate::core::settings::Settings,
    profile: &crate::core::model_profile::ModelProfile,
    shape: &crate::core::loopshape::LoopShape,
    sub: &Option<crate::core::subagent::SubagentType>,
    history: &Option<Vec<HistMsg>>,
    system: &str,
    message: &str,
    volatile: &mut String,
    depth: u8,
    sid: &String,
) -> Fitted {
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    // ---- Context budget -------------------------------------------------------
    // A session's history grows forever and a single turn can pile up 24 steps of
    // tool output, so both were sent whole and the turn eventually died on a
    // context-overflow error (or, on Ollama, silently lost its own instructions off
    // the front of the window). Trim the history to whole recent turns, summarize
    // what that drops, and squeeze historical tool results inside the loop.
    // The model's own window is the ceiling the configured budget is taken against —
    // H4: 48,000 configured against a 16,384 allocation is a budget that does not
    // exist, and Ollama answers an overflow by dropping the front of the prompt.
    let budget = crate::core::context::Budget::for_model(&settings, &profile);
    let (history_msgs, dropped) =
        crate::core::context::trim_turns(hist_json(&history, &profile), budget.max_turns, budget.max_tokens.saturating_sub(estimate_fixed(&system, &message)));
    // Watch the far side of the boundary (HARNESS-RESEARCH-2026 §3). Opened here
    // because this is the only place that knows what was taken away; scored after
    // the turn, from messages that exist anyway, rather than threaded through
    // dispatch to count two integers.
    // One guess in flight for the whole turn (HARNESS-RESEARCH-2026 §5).
    let speculator = crate::core::speculation::Speculator::new();
    let mut compaction_watch = (!dropped.is_empty()).then(|| {
        crate::core::compaction::Watch::open(
            crate::core::compaction::Boundary {
                turns_dropped: dropped.len(),
                chars_dropped: dropped.iter().map(|d| d.to_string().len()).sum(),
                summarized: false,
            },
            &dropped,
        )
    });
    if !dropped.is_empty() {
        emit("text", format!("Summarizing {} earlier messages to stay inside the context window…", dropped.len()));
        let summary = crate::core::context::summarize_dropped(&settings, &dropped).await;
        if !summary.is_empty() {
            // FIFO and summary are different arms with very different measured
            // outcomes, so the record has to be able to tell them apart.
            if let Some(w) = compaction_watch.as_mut() {
                w.set_summarized(true);
            }
            // Volatile: it grows as the conversation is trimmed, so it belongs after
            // the breakpoint rather than in front of everything stable.
            volatile.push_str("\n\n## Earlier in this conversation\n\nThese turns were summarized to fit the context window. Treat them as established context:\n\n");
            volatile.push_str(&summary);
        }
        // What falls out of the context window is exactly what would otherwise be
        // forgotten, so this is the moment long-term memory is written. Both tiers
        // are written from the same event: the transcript excerpt is the evidence
        // the nightly consolidation reads, the summary is the readable view of it.
        // Writing only the summary — which is what this used to do — made the core
        // memory a distillation of a distillation, with the words themselves kept
        // nowhere. A subagent's context is not the user's conversation, so it never
        // writes either tier.
        if depth == 0 {
            crate::core::journal::Journal::new(&state.memory.vault_path)
                .record_trim(&crate::core::journal::today(), &dropped, &summary, &sid);
        }
    }
    // A subagent's task is already bounded, so it gets half the parent's steps.
    // Tier-aware: a small model that has not finished in a dozen steps is usually
    // looping rather than converging, and every step is another wait for the user.
    // Only ever lower than the user's setting — see `LoopShape::for_model`.
    let max_steps = if sub.is_some() { crate::core::subagent::step_budget(shape.max_steps) } else { shape.max_steps };
    Fitted { budget, history_msgs, speculator, compaction_watch, max_steps }
}
