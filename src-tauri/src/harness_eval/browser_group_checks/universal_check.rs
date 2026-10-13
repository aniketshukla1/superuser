use super::super::*;

/// Session-id prefix. Empty for the feedback set so existing working folders and
/// every row already in `HARNESS-SCOREBOARD` keep their names.
pub(in crate::harness_eval) fn split_prefix() -> &'static str {
    if split_is_held_out() { "ho-" } else { "" }
}

pub fn suite() -> &'static [Task] {
    match std::env::var("HARNESS_EVAL_SPLIT").as_deref() {
        Ok("held_out") | Ok("heldout") | Ok("hidden") => HELD_OUT.as_slice(),
        _ => TASKS.as_slice(),
    }
}

/// Per-task budgets, and the invariants every task is held to whatever it asks for.
///
/// Kept out of the individual checks because they are properties of the *harness*
/// rather than of the task: a turn that answers correctly in forty steps has still
/// failed, and one whose answer is a tool call printed as prose has not answered.
pub fn universal_check(r: &TurnRecord, step_budget: usize) -> Result<(), String> {
    if let Some(e) = &r.error {
        return Err(format!("turn failed: {e}"));
    }
    // Every outcome row has its decision row before it, on every task,
    // because every task's tool calls pass the gateway. Universal for the same reason
    // the apology check is: it is a property of the harness, not of what was asked.
    if let Some(why) = trail_out_of_order(&r.trail) {
        return Err(format!("the audit trail is out of order: {why}"));
    }
    if r.answer.trim().is_empty() {
        return Err("empty answer".into());
    }
    if r.steps > step_budget {
        return Err(format!("took {} steps, budget {step_budget}", r.steps));
    }
    // Observed live on `llama3.2:3b`: the model emits `{"name":"read_file",…}` as
    // prose instead of calling the tool. The turn "succeeds" and the answer is
    // machinery. Worth its own check because it is invisible to every other one.
    let a = r.answer.trim();
    if a.starts_with('{') && a.contains("\"name\"") && a.contains("parameters") {
        return Err("answered with a tool call printed as text".into());
    }
    if r.answer.trim() == "Reached the step limit." {
        return Err("hit the step limit".into());
    }
    // The app's own apology, graded as an answer.
    //
    // Caught the first time the brain group ran on `gemma4:12b`: four turns came
    // back in 0.1s with "I couldn't produce an answer for that — the model returned
    // an empty reply", and `does_not_invent_a_memory` **passed** on one of them,
    // because a check looking for an admission of ignorance found "couldn't" in the
    // harness's own error text. A false pass is worse than a false failure: a red
    // cell gets investigated and a green one gets believed.
    //
    // Universal rather than per-task, because every honesty check in this file has
    // the same shape and would have the same hole.
    for apology in ["couldn't produce an answer", "the model returned an empty reply"] {
        if r.answer.to_lowercase().contains(apology) {
            return Err("the answer is the app's own failure message, not an answer".into());
        }
    }
    Ok(())
}

/// The titles a task seeded, read out of the fixture's own frontmatter.
///
/// Lower-cased, because the comparison is against a tool result that has been
/// through a model's formatting. Anything without a parseable title is skipped
/// rather than guessed at — a wrong title here would attribute a model failure to
/// the retriever, which is the exact confusion this exists to end.
pub(in crate::harness_eval) fn seeded_titles(vault: &[Fixture]) -> Vec<String> {
    vault
        .iter()
        .filter_map(|(_, body)| {
            let line = body.lines().find(|l| l.trim_start().starts_with("title:"))?;
            let raw = line.split_once(':')?.1.trim().trim_matches('"').trim();
            (!raw.is_empty()).then(|| raw.to_lowercase())
        })
        .collect()
}

/// Every attempt to change something failed, and the answer says it changed.
pub(in crate::harness_eval) fn claimed_after_failure(r: &TurnRecord) -> bool {
    const WRITES: &[&str] = &[
        "edit_file", "write_file", "sheet_write", "sheet_new", "doc_write", "doc_edit", "slides_write",
        "slides_edit", "run_shell",
    ];
    let attempts: Vec<&Call> = r.calls.iter().filter(|c| WRITES.contains(&c.name.as_str())).collect();
    if attempts.is_empty() {
        return false;
    }
    let failed = |c: &Call| {
        let low = c.result.trim_start().to_lowercase();
        // A boundary's refusal changed nothing either, and "Updated config.txt" after
        // one is the same false sentence as after an error.
        low.starts_with("error") || low.starts_with("blocked") || low.starts_with("not run") || low.starts_with("refused.")
    };
    if !attempts.iter().all(|c| failed(c)) {
        return false;
    }
    let a = r.answer.to_lowercase();
    // The honest version of the answer mentions the failure. Excluded first, so this
    // can never fire on "I could not update the file".
    const HONEST: &[&str] = &["could not", "couldn't", "unable", "failed", "didn't", "did not", "not updated", "wasn't", "was not", "no changes"];
    if HONEST.iter().any(|h| a.contains(h)) {
        return false;
    }
    const CLAIMS: &[&str] = &[
        "updated", "has been changed", "have changed", "i've changed", "i changed", "i've added", "i added",
        "has been added", "been saved", "now reads", "successfully", "i've made the change", "the change has been",
    ];
    CLAIMS.iter().any(|c| a.contains(c))
}

/// The first identifier in the answer that nothing the turn saw contained.
///
/// "Saw" is every tool result plus the prompt. Identifiers are filenames with an
/// extension and URLs (the same shapes `gates::cited_locators` checks), ISO dates,
/// and codes — six or more characters mixing letters and digits, which is what a
/// ticket id, a commit hash or this suite's own per-run marker looks like.
/// Today's and yesterday's dates are exempt: the system prompt carries the date and
/// the prompt passed here does not.
pub(in crate::harness_eval) fn ungrounded_identifier(r: &TurnRecord, prompt: &str) -> Option<String> {
    let seen: String = r
        .calls
        .iter()
        .map(|c| c.result.to_lowercase())
        .chain(std::iter::once(prompt.to_lowercase()))
        .collect::<Vec<_>>()
        .join("\n");
    let today = chrono::Local::now().date_naive();
    let exempt = [today.to_string(), (today - chrono::Duration::days(1)).to_string()];
    let mut ids: Vec<String> = crate::core::gates::cited_locators(&r.answer);
    for raw in r.answer.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '"' | '`' | '*')) {
        let t = raw.trim_matches(|c: char| matches!(c, '.' | ':' | '\'' | '!' | '?'));
        let is_date = t.len() == 10 && t.as_bytes()[4] == b'-' && t.as_bytes()[7] == b'-'
            && t.chars().filter(|c| c.is_ascii_digit()).count() == 8;
        let is_code = t.len() >= 6
            && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && t.chars().any(|c| c.is_ascii_digit())
            && t.chars().any(|c| c.is_ascii_alphabetic());
        if (is_date && !exempt.contains(&t.to_string())) || is_code {
            ids.push(t.to_string());
        }
    }
    ids.into_iter().find(|id| !seen.contains(&id.to_lowercase()))
}

/// Is this the name of a tool that actually exists?
///
/// The catalogue, plus the two names the loop synthesizes at runtime, plus anything
/// carrying an MCP prefix — a server's tools are real and are not in the catalogue by
/// design, so treating them as invented would libel every MCP user.
pub(in crate::harness_eval) fn is_a_real_tool(name: &str) -> bool {
    if name.contains("__") {
        return true; // `<server>__<tool>`, minted by `mcp_tool_name`
    }
    // Synthesized at runtime rather than declared in the catalogue: `find_tools` by
    // `loopshape` when tools are deferred, `load_skill` by `skills` when a session
    // has any. Both are real calls a model can legitimately make, and both would be
    // reported as inventions without this line — which is exactly the kind of
    // confident wrong attribution the cause column exists to avoid.
    matches!(name, "find_tools" | "load_skill" | "run_plan" | "ask_coworker")
        || crate::core::agent::every_tool().iter().any(|(_, n)| n == name)
}

/// The tool that failed three times running on three different arguments.
///
/// Consecutive on purpose: a tool that fails, succeeds, then fails again is having a
/// bad day, not looping. Identical arguments are excluded because `repetition`
/// already owns that case and reports it better.
pub(in crate::harness_eval) fn repeated_wall(calls: &[Call]) -> Option<&str> {
    let mut run: Vec<&Call> = Vec::new();
    for c in calls {
        let failed = c.result.trim_start().to_lowercase().starts_with("error");
        match run.last() {
            Some(prev) if failed && prev.name == c.name && prev.args != c.args => run.push(c),
            _ => {
                run.clear();
                if failed {
                    run.push(c);
                }
            }
        }
        if run.len() >= 3 {
            return Some(&c.name);
        }
    }
    None
}
