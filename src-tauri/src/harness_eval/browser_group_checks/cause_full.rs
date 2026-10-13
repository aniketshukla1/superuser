use super::super::*;

/// The full attribution, told the prompt the turn was given.
///
/// `prompt` is `None` everywhere the prompt is not known, and then the ungrounded
/// check is skipped rather than run against an empty string — which would report
/// every filename the user typed as an invention.
pub fn cause_full(
    r: &TurnRecord,
    group: &str,
    step_budget: usize,
    failed: bool,
    vault: &[Fixture],
    prompt: Option<&str>,
) -> Option<Cause> {
    if !failed || r.unreachable.is_some() {
        return None;
    }
    if r.error.is_some() {
        return Some(Cause::Provider);
    }
    if trail_out_of_order(&r.trail).is_some() {
        return Some(Cause::TrailOutOfOrder);
    }
    let a = r.answer.trim();
    if a.is_empty() {
        return Some(Cause::EmptyAnswer);
    }
    // The app's apology for an empty reply is an empty answer wearing prose. Above
    // the tool checks on purpose: a turn that never got a reply also never called a
    // tool, and reporting that as `no tool` blames selection for what is really the
    // provider handing back nothing. Seen live — Ollama returned an empty message
    // for `gemma4:12b` on every request, and the board read it as four selection
    // failures.
    let lower = a.to_lowercase();
    if lower.contains("couldn't produce an answer") || lower.contains("the model returned an empty reply") {
        return Some(Cause::EmptyAnswer);
    }
    if r.steps > step_budget || a == "Reached the step limit." {
        return Some(Cause::StepCliff);
    }
    if a.starts_with('{') && a.contains("\"name\"") && a.contains("parameters") {
        return Some(Cause::ToolCallAsText);
    }
    // A call whose arguments never parsed arrives as `{}` (see `core::argrepair`),
    // and a call the loop refused says so in the result it hands back.
    let arg_trouble = r.calls.iter().any(|c| {
        let args = c.args.trim();
        // `{}` is a parse failure only for a tool that wanted something. `browser_read`
        // and `browser_snapshot` take nothing, and are called that way every time.
        let empty = (args.is_empty() || args == "{}") && !crate::core::agent::takes_no_arguments(&c.name);
        let rejected = c.result.starts_with("error:")
            && (c.result.contains("argument") || c.result.contains("required"));
        empty || rejected
    });
    if arg_trouble {
        return Some(Cause::ArgumentHandling);
    }
    // A name that exists nowhere. Above the selection check because "it called
    // something that is not a tool" explains more than "it called no real tool".
    if r.calls.iter().any(|c| !is_a_real_tool(&c.name)) {
        return Some(Cause::ToolHallucination);
    }
    if needs_a_tool(group) && r.calls.is_empty() {
        return Some(Cause::NoToolCalled);
    }
    if went_around_a_refusal(r).is_some() {
        return Some(Cause::WentAroundARefusal);
    }
    // Three failures of one tool with three different arguments: the retries are the
    // same mistake, not a search. Below argument handling, which explains the first
    // failure better when that is what it was.
    if repeated_wall(&r.calls).is_some() {
        return Some(Cause::ReentrantFailure);
    }
    if claimed_after_failure(r) {
        return Some(Cause::ClaimedAfterFailure);
    }
    if let Some(p) = prompt {
        if ungrounded_identifier(r, p).is_some() {
            return Some(Cause::UngroundedClaim);
        }
    }
    // The retriever's side of a brain failure. A task that seeds memories and then
    // calls `recall_memory` has told us what should have come back: the seeded files
    // are the only place the answer lives. If none of their titles appear in any
    // recall result, the retriever missed and the model was never given a chance —
    // which is a defect in `embeddings.rs`, not in the model, and the two were
    // indistinguishable on the board until now.
    if !vault.is_empty() {
        let recalls: String = r
            .calls
            .iter()
            .filter(|c| c.name == "recall_memory")
            .map(|c| c.result.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");
        if !recalls.is_empty() {
            let titles = seeded_titles(vault);
            if !titles.is_empty() && !titles.iter().any(|t| recalls.contains(t)) {
                return Some(Cause::RetrievalMiss);
            }
        }
    }
    Some(Cause::Model)
}

/// How a task ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    /// No measurement was obtained. Reported as a blank, never as a failure, and
    /// excluded from the denominator — a run that could not be measured says
    /// nothing about the model.
    NoResult,
}

/// One task's outcome on one model.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub task: &'static str,
    pub group: &'static str,
    pub model: String,
    /// Which repeat this came from, 0-based.
    ///
    /// Present because a single run of a single task is not a measurement — measured
    /// the hard way: the same task on the same model with identical configuration
    /// took 16 steps once and 24 the next time, and the difference was reported as a
    /// result. Repetition is a feature of the tool now rather than something a person
    /// remembers to do.
    #[serde(default)]
    pub run: usize,
    pub verdict: Verdict,
    pub why: Option<String>,
    /// Which side a failure came from. `None` on a pass or a blank.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
    pub steps: usize,
    pub seconds: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    /// Carried from the run: why it isn't comparable with the board's rows. The report marks
    /// it with a dagger and says why beneath the summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_comparable: Option<String>,
}

/// Score one record against one task.
pub fn score(task: &Task, model: &str, r: &TurnRecord, step_budget: usize) -> Outcome {
    if let Some(e) = &r.unreachable {
        return Outcome {
            task: task.id,
            group: task.group,
            model: model.to_string(),
            run: 0,
            verdict: Verdict::NoResult,
            why: Some(e.clone()),
            cause: None,
            steps: r.steps,
            seconds: r.seconds,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cost_usd: r.cost_usd,
            not_comparable: r.not_comparable.clone(),
        };
    }
    let why = universal_check(r, step_budget)
        .and_then(|()| (task.check)(r))
        .err();
    Outcome {
        task: task.id,
        group: task.group,
        model: model.to_string(),
        run: 0,
        verdict: if why.is_none() { Verdict::Pass } else { Verdict::Fail },
        cause: cause_full(
            r,
            task.group,
            step_budget,
            why.is_some(),
            task.vault,
            // The standing instructions are part of what the turn was told, so a code
            // they carry is not an invention when it shows up in the answer.
            Some(&format!("{}\n{}", task.prompt, task.standing).replace(MARKER_SLOT, &r.marker)),
        ),
        why,
        steps: r.steps,
        seconds: r.seconds,
        input_tokens: r.input_tokens,
        output_tokens: r.output_tokens,
        cost_usd: r.cost_usd,
        not_comparable: r.not_comparable.clone(),
    }
}
