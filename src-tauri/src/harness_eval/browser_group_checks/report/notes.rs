use super::*;

/// The notes under the table: runs not comparable with the board, every failure by name,
/// and the tasks that produced no result.
pub(super) fn notes(outcomes: &[Outcome], models: &[&str], out: &mut String) {
    // A run under a setup the board's rows weren't measured under is a real
    // result of a different setup: counted above, marked, and explained here, so a reader doesn't
    // set it beside the rest.
    let flagged: Vec<&Outcome> = outcomes.iter().filter(|o| o.not_comparable.is_some()).collect();
    if !flagged.is_empty() {
        out.push_str("\n† Not comparable with the board's other rows:\n");
        for m in models {
            let of_m: Vec<&&Outcome> = flagged.iter().filter(|o| o.model == *m).collect();
            if of_m.is_empty() {
                continue;
            }
            let total = outcomes.iter().filter(|o| o.model == *m).count();
            let mut why: Vec<&str> = of_m.iter().filter_map(|o| o.not_comparable.as_deref()).collect();
            why.sort_unstable();
            why.dedup();
            out.push_str(&format!("- `{m}`: {} of {total} runs {}.\n", of_m.len(), why.join("; ")));
        }
        out.push_str("Run the sweep on a scratch instance (`SUPERUSER_DATA_DIR`) for rows that are.\n");
    }

    // Every failure, named. A table of crosses says a model is worse; only this says
    // how, and "how" is the thing that turns a score into a fix.
    let failures: Vec<&Outcome> = outcomes.iter().filter(|o| o.verdict == Verdict::Fail).collect();
    if !failures.is_empty() {
        // Attribution before the list. A reader who stops after one paragraph should
        // leave knowing which failures are ours, because that is the only part of
        // this report anyone here can act on.
        let mut counts: Vec<(Cause, usize)> = Vec::new();
        for f in &failures {
            let c = f.cause.unwrap_or(Cause::Model);
            match counts.iter_mut().find(|(k, _)| *k == c) {
                Some((_, n)) => *n += 1,
                None => counts.push((c, 1)),
            }
        }
        // Most common first, and ours before the model's at equal counts — the
        // ordering encodes what to look at, not just how much of it there is.
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.ours().cmp(&a.0.ours())));
        let ours: usize = counts.iter().filter(|(c, _)| c.ours()).map(|(_, n)| n).sum();
        out.push_str(&format!(
            "\n**Where {} failure(s) came from** — {ours} on our side, {} model-side\n\n",
            failures.len(),
            failures.len() - ours
        ));
        for (c, n) in &counts {
            out.push_str(&format!(
                "* {n} × {}{}\n",
                c.label(),
                if c.ours() { "" } else { " (not a harness defect)" }
            ));
        }

        out.push_str("\n**Failures**\n\n");
        for f in failures {
            out.push_str(&format!(
                "* `{}` on {} — [{}] {}\n",
                f.task,
                f.model,
                f.cause.map(|c| c.label()).unwrap_or("?"),
                f.why.as_deref().unwrap_or("?")
            ));
        }
    }
    let missing: Vec<&Outcome> = outcomes.iter().filter(|o| o.verdict == Verdict::NoResult).collect();
    if !missing.is_empty() {
        out.push_str("\n**Not measured** (the driver never got an answer; not scored)\n\n");
        for f in missing {
            out.push_str(&format!(
                "* `{}` on {} — {}\n",
                f.task,
                f.model,
                f.why.as_deref().unwrap_or("?")
            ));
        }
    }
}
