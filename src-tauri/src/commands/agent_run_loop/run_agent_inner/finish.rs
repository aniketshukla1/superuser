use super::*;

/// The turn's report: the answer, what it cost and took, and what it left behind.
pub(super) fn finish(t: Turn<'_>, ran: Ran, turn_began: std::time::Instant, run_started: Option<std::time::SystemTime>) -> AgentRunOut {
    let Turn { app, sink, exec, working_dir, model, persona, tally, .. } = t;
    let emit = |kind: &str, text: String| turn_event(app, &sink, kind, text);
    let Ran { answer: final_answer, usage: final_usage, wire: final_wire } = ran;
    emit("done", "Done.".into());
    let todos = exec.todo.lock().unwrap().clone();
    // Only what this turn touched, when the folder is the user's rather than the
    // run's own — see `artifacts_since`.
    let artifacts: Vec<String> = crate::core::agent_exec::artifacts_since(&working_dir, run_started)
        .into_iter()
        // The session record and the run's own journal share the folder with the
        // deliverables but are not deliverables — a turn that produced nothing
        // should say so, not list its own bookkeeping.
        .filter(|f| f != "session.json" && f != crate::core::runlog::FILE)
        .collect();
    let final_usage = final_usage.filter(|u| !u.is_empty()).map(|mut u| {
        // Stamp the model onto the usage record. The GUI groups usage `byModel` and
        // has always had a per-model table with an "Unknown model" branch — and every
        // row took it, because this sidecar is what it reads and nothing ever put a
        // model in it. Set here, at the one place that knows which model the turn
        // actually resolved to (an explicit pick, or the effective default).
        u.model = Some(model.clone());
        u
    });
    let measure = TurnMeasure {
        steps: tally.steps_taken,
        seconds: turn_began.elapsed().as_secs_f64(),
        // Priced against the model the turn actually ran on. `None` propagates: a
        // model we have no rate for must read as unknown, never as free.
        cost_usd: final_usage.as_ref().and_then(|u| crate::core::pricing::cost_of(&model, u)),
        cache_hit_rate: final_usage.as_ref().and_then(|u| u.cache_hit_rate()),
        tool_seconds: tally.tool_wall.as_secs_f64(),
        tool_seconds_summed: tally.tool_summed.as_secs_f64(),
        tools_parallel: tally.tools_parallel,
    };
    crate::trace!(
        "turn",
        "done steps={} seconds={:.1} usage={:?} cost={:?}",
        measure.steps,
        measure.seconds,
        final_usage,
        measure.cost_usd
    );
    crate::trace!(
        "turn",
        "transcript {}",
        match &final_wire {
            Some(w) => format!("kept {} messages, shape {:?}", w.messages.len(), w.shape),
            None => "none — no tool calls this turn".to_string(),
        }
    );
    AgentRunOut {
        answer: final_answer,
        usage: final_usage,
        wire: final_wire,
        measure: Some(measure),
        provider: model,
        persona,
        todos,
        artifacts,
        charts: exec.charts_drawn(),
        working_dir: working_dir.to_string_lossy().into_owned(),
    }
}
