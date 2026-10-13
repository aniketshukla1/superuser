use super::super::*;

mod notes;
use notes::*;

/// The scoreboard, as a markdown table.
///
/// Written to be pasted into HARNESS-BASELINE, which is where every claim in
/// HARNESS-PLAN is supposed to be checked.
pub fn report(outcomes: &[Outcome]) -> String {
    let mut models: Vec<&str> = outcomes.iter().map(|o| o.model.as_str()).collect();
    models.dedup();
    models.sort_unstable();
    models.dedup();

    // `dedup` only collapses *adjacent* duplicates, and outcomes arrive model-major —
    // t1..t19 for one model, then t1..t19 for the next — so nothing was adjacent and
    // the table printed all 19 rows once per model. `models` above got away with it
    // only because it sorts in between. Order is preserved here rather than sorted:
    // the task order is the suite's own, which groups related checks together.
    let mut seen = std::collections::HashSet::new();
    let tasks: Vec<(&str, &str)> = outcomes
        .iter()
        .map(|o| (o.task, o.group))
        .filter(|t| seen.insert(t.0))
        .collect();

    let mut out = String::new();
    out.push_str("| task | group |");
    for m in &models {
        out.push_str(&format!(" {m} |"));
    }
    out.push_str("\n|---|---|");
    for _ in &models {
        out.push_str("---|");
    }
    out.push('\n');

    for (task, group) in &tasks {
        out.push_str(&format!("| `{task}` | {group} |"));
        for m in &models {
            // Across every repeat, because one run of one task is not a measurement.
            // A cell that read ✅ or ❌ hid exactly the variance that made a 16-step
            // run and a 24-step run of the same task look like a result.
            let runs: Vec<&Outcome> =
                outcomes.iter().filter(|o| o.task == *task && o.model == *m).collect();
            let scored: Vec<&&Outcome> =
                runs.iter().filter(|o| o.verdict != Verdict::NoResult).collect();
            let passed = scored.iter().filter(|o| o.verdict == Verdict::Pass).count();
            let cell = if scored.is_empty() {
                "–".to_string()
            } else if scored.len() == 1 {
                // A single run still reads as a tick or a cross; adding "1/1" to every
                // cell of an unrepeated sweep would be noise.
                if passed == 1 { "✅".into() } else { "❌".into() }
            } else if passed == scored.len() {
                format!("✅ {passed}/{}", scored.len())
            } else if passed == 0 {
                format!("❌ 0/{}", scored.len())
            } else {
                // The interesting case, and the one a tick would have hidden.
                format!("⚠ {passed}/{}", scored.len())
            };
            // M-5: a run under a setup the board wasn't measured under, explained below.
            let dagger = if runs.iter().any(|o| o.not_comparable.is_some()) { "†" } else { "" };
            out.push_str(&format!(" {cell}{dagger} |"));
        }
        out.push('\n');
    }

    // Three statistics, not one. HARNESS-RESEARCH-2026 §6b: the same paper that
    // measured our repeat count as under-powered also measured pass@k − pass^k gaps
    // of up to 24.9 points, which is to say a quarter of apparent capability can be
    // stochastic exploration. The `⚠ 2/3` cells above were already the honest
    // measurement; the summary row was the dishonest one, because a total of ticks
    // reads as pass@1 and is not. A change that turns six `⚠ 2/3` into `✅ 3/3` is a
    // large reliability win that the old total rendered as +6.
    out.push_str("\n| model | passed | pass@1 | pass^k | steps (median) | tokens in | seconds | cost |\n|---|---|---|---|---|---|---|---|\n");
    for m in &models {
        let all: Vec<&Outcome> = outcomes.iter().filter(|o| o.model == *m).collect();
        let unmeasured = all.iter().filter(|o| o.verdict == Verdict::NoResult).count();
        // Scored against what was actually measured. Counting a task the driver
        // never got an answer for as a failure would report the harness's own
        // timeout as the model's score.
        let rows: Vec<&Outcome> = all.iter().copied().filter(|o| o.verdict != Verdict::NoResult).collect();
        let passed = rows.iter().filter(|o| o.verdict == Verdict::Pass).count();
        let mut steps: Vec<usize> = rows.iter().map(|o| o.steps).collect();
        steps.sort_unstable();
        let median = steps.get(steps.len() / 2).copied().unwrap_or(0);
        let tokens: u64 = rows.iter().map(|o| o.input_tokens).sum();
        let secs: f64 = rows.iter().map(|o| o.seconds).sum();
        // `None` propagates: a model we have no rate for reads as unknown, never free.
        let cost: Option<f64> = rows
            .iter()
            .map(|o| o.cost_usd)
            .try_fold(0.0, |acc, c| c.map(|c| acc + c));
        // Per-run scores when there is more than one, because a mean with no spread
        // beside it is the shape of claim this file exists to stop.
        let mut per_run: Vec<usize> = Vec::new();
        let runs = all.iter().map(|o| o.run).max().map(|n| n + 1).unwrap_or(1);
        if runs > 1 {
            for r in 0..runs {
                per_run.push(
                    all.iter()
                        .filter(|o| o.run == r && o.verdict == Verdict::Pass)
                        .count(),
                );
            }
        }
        let spread = if per_run.len() > 1 {
            let lo = per_run.iter().min().copied().unwrap_or(0);
            let hi = per_run.iter().max().copied().unwrap_or(0);
            let each: Vec<String> = per_run.iter().map(|n| n.to_string()).collect();
            format!(" — runs {} (spread {})", each.join(", "), hi - lo)
        } else {
            String::new()
        };
        // pass@1: the share of measured task-runs that passed. What the old total
        // was silently being read as.
        let pass_at_1 = if rows.is_empty() { 0.0 } else { passed as f64 / rows.len() as f64 };

        // pass^k: the share of TASKS that passed every run they were measured on.
        // A task with a blank in one run is judged on the runs that produced an
        // answer — the "a blank is not a zero" rule this file applies everywhere
        // else — and a task with no measured run at all counts toward neither.
        let mut task_ids: Vec<&str> = rows.iter().map(|o| o.task).collect();
        task_ids.sort_unstable();
        task_ids.dedup();
        let mut all_pass = 0usize;
        for t in &task_ids {
            let runs_of_t: Vec<&&Outcome> = rows.iter().filter(|o| o.task == *t).collect();
            if !runs_of_t.is_empty() && runs_of_t.iter().all(|o| o.verdict == Verdict::Pass) {
                all_pass += 1;
            }
        }
        let pass_k = if task_ids.is_empty() { 0.0 } else { all_pass as f64 / task_ids.len() as f64 };

        let dagger = if all.iter().any(|o| o.not_comparable.is_some()) { "†" } else { "" };
        out.push_str(&format!(
            "| {m}{dagger} | **{passed}/{}**{}{spread} | {:.0}% | {}/{} ({:.0}%) | {median} | {tokens} | {secs:.0} | {} |\n",
            rows.len(),
            if unmeasured > 0 { format!(" ({unmeasured} not measured)") } else { String::new() },
            pass_at_1 * 100.0,
            all_pass,
            task_ids.len(),
            pass_k * 100.0,
            cost.map(|c| format!("${c:.4}")).unwrap_or_else(|| "—".into())
        ));
    }

    notes(outcomes, &models, &mut out);
    out
}
