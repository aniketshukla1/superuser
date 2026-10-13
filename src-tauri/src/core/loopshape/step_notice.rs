use super::*;

// ---------------------------------------------------------------------------
// The step budget, made visible
// ---------------------------------------------------------------------------

/// What to tell the model about the steps it has left, if anything.
///
/// HARNESS-PLAN Phase 7: today a turn that runs out ends with `"Reached the step
/// limit."` — a failure the model was never warned about. It spends its last step on
/// another tool call, the loop stops, and the user gets a sentence about a limit
/// instead of an answer.
///
/// The scoreboard measured what that costs. `gemma4:12b` looped for **24 steps and
/// 1,645 seconds** on `grep_before_reading_everything` — a task another model answers
/// in one — and produced nothing at all. Twenty-seven minutes for a sentence about a
/// limit is a worse outcome than a wrong answer given in ten seconds.
///
/// Silent while there is room, on purpose: a budget restated every step is noise a
/// model learns to skip, and it is paid for in tokens on every step of every turn.
///
/// **The milestones scale with the budget, and the first version's did not.** It warned
/// at a fixed three-remaining, which on the default 24-step budget is step 22 — far too
/// late to change what a model does. Graded, it converted gemma4's silent 24-step loop
/// into a 23-step loop that at least answered: nothing became something, and the
/// eighteen wasted minutes stayed wasted. A budget is only steerable while there is
/// enough of it left to steer with.
///
/// So the notice fires when the *remaining* count crosses a fraction of the whole —
/// half, then a quarter, then the last step. Three notices per turn whatever the
/// budget, each on exactly the step that crosses its mark, and they collapse sensibly
/// when the budget is small enough that the fractions coincide.
///
/// `used` is steps already completed, so `max - used` counts the step about to run.
pub fn step_notice(used: usize, max: usize) -> Option<String> {
    let left = max.saturating_sub(used);
    // The loop is already over; there is nobody left to tell.
    if left == 0 {
        return None;
    }
    if left == 1 {
        return Some(
            "This is your final step — no more tools will run. Answer now from what you \
             already have, and say plainly what you could not finish."
                .into(),
        );
    }
    // Checked before the halfway mark so that on a budget small enough for the two to
    // coincide, the more urgent wording is the one that survives.
    if left == max / 4 {
        return Some(format!(
            "Only {left} of this turn's {max} steps are left. Stop gathering and start \
             answering with what you have."
        ));
    }
    if left == max / 2 {
        return Some(format!(
            "Half of this turn's {max} steps are gone and {left} remain. If you are \
             still searching, narrow it down now rather than widening."
        ));
    }
    None
}

/// Whether the step about to run is the last one the loop will allow.
///
/// The loop offers no tools on that step, which is what makes [`step_notice`]'s
/// promise true rather than a hint. A warning the model can ignore is a warning it
/// will ignore; removing the tools converts "ran out of steps" into "answered with
/// what it had", which is the difference between nothing and something.
pub fn is_final_step(used: usize, max: usize) -> bool {
    max.saturating_sub(used) <= 1
}

// ---------------------------------------------------------------------------
// When the model says nothing at all
// ---------------------------------------------------------------------------

/// What to say to a model whose reply was empty — no text, no tool call.
///
/// Measured repeatedly on local models: `custom:qwen2.5:7b-instruct` returned
/// `{"content":"","role":"assistant"}` on three scoreboard tasks, and `gemma4:12b`
/// converged at 16 of 24 steps and did the same. Verified against raw curl that the
/// harness accumulates correctly and the model genuinely sends nothing — so this is
/// the model losing its footing, and the cheapest repair is to say so and ask again.
///
/// Once per turn. A model that answers emptily twice will answer emptily ten times,
/// and each attempt is another wait for the user.
pub const EMPTY_REPLY_NUDGE: &str =
    "Your last reply was completely empty — no text and no tool call. Answer the \
     question directly now, in words. If something is blocking you, say what it is.";

/// The answer to record for a turn that ended with the model having said nothing.
///
/// `"Done."` is a fair summary of a turn that ran tools and had nothing left to add.
/// It is a lie about a turn where **nothing happened at all** — and it is the more
/// damaging kind of lie, because it reads to the user exactly like success. The
/// scoreboard could not tell those two apart either; that is why three tasks scored a
/// polite "Done." against prompts they never addressed.
pub fn empty_answer(tools_ran: bool) -> String {
    if tools_ran {
        "Done.".into()
    } else {
        "I couldn't produce an answer for that — the model returned an empty reply. \
         Try rephrasing, or switch models in the composer."
            .into()
    }
}

/// The name of the tool that gets a withheld tool back.
pub const FIND_TOOLS: &str = "find_tools";

/// The tool a narrowed loop always carries.
///
/// Without it, narrowing is a capability cut: a model on a small local backend simply
/// could not draw a chart or read a spreadsheet any more. With it, the catalogue is
/// deferred rather than removed — which is what Anthropic's tool search does natively
/// and what nothing else offers.
pub fn find_tools_spec(held: &[ToolSpec]) -> ToolSpec {
    // Naming the *capabilities* rather than the tools: it is a fraction of the tokens
    // of the schemas themselves, and it is what a model needs to know that asking is
    // worth a step.
    let mut caps: Vec<&str> = held.iter().map(|t| t.capability.as_str()).collect();
    caps.sort_unstable();
    caps.dedup();
    ToolSpec {
        name: FIND_TOOLS.into(),
        description: format!(
            "Get tools that are not listed above. More tools exist for: {}. \
             Call this with what you are trying to do, then use the tools it returns.",
            caps.join(", ")
        ),
        schema: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "What you are trying to do, e.g. \"read a spreadsheet\"." }
            },
            "required": ["query"]
        }),
        risk: crate::core::agent::RiskClass::Read,
        capability: "find_tools".into(),
    }
}

/// Split a string into the words worth matching on.
pub(super) fn words_of(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

/// Whether two words refer to the same thing, near enough.
///
/// Substring in **either** direction. Measured against the live loop: a model asked
/// for a "charting tool" and `find` returned nothing, because `"chart".contains(
/// "charting")` is false — one-directional matching misses every query phrased as a
/// gerund or a plural, which is how models phrase most of them. The length floor is
/// what stops "the" matching everything.
pub(super) fn akin(a: &str, b: &str) -> bool {
    a == b || (a.len() >= 4 && b.contains(a)) || (b.len() >= 4 && a.contains(b))
}
