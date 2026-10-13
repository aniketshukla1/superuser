use super::*;

/// One outcome's line on the board and, for a failure, what the turn called and — with
/// `HARNESS_EVAL_JUDGE` — whether a second model finds the answer unsupported.
#[allow(clippy::too_many_arguments)]
pub(super) async fn print_outcome(o: &Outcome, task: &Task, model: &str, rec: &TurnRecord, settings: &Value, marker: &str, judged: &mut usize, judged_unsupported: &mut usize) {
    eprintln!(
        "  {} {:<32} {:<28} {:>2} steps {:>6.1}s {}",
        match o.verdict {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::NoResult => "----",
        },
        task.id,
        model,
        o.steps,
        o.seconds,
        o.why.as_deref().unwrap_or("")
    );
    // `HARNESS_EVAL_TRACE_FAILURES=1`: print what a failing turn actually
    // called. Added because reading a failure used to mean archaeology —
    // the write-path tasks sat at 0/3 through a whole day of work, and the
    // one number that mattered (every `edit_file` call erroring, then the
    // model stopping) was only visible by folding the per-tool table by
    // hand. The arguments are what say *why*, and nothing printed them.
    if o.verdict == Verdict::Fail
        && std::env::var("HARNESS_EVAL_TRACE_FAILURES").is_ok_and(|v| v == "1")
    {
        for (i, c) in rec.calls.iter().enumerate() {
            eprintln!(
                "      {i}. {}({})\n         -> {}",
                c.name,
                c.args.chars().take(240).collect::<String>(),
                c.result.replace('\n', " | ").chars().take(240).collect::<String>()
            );
        }
        eprintln!("      answer: {:?}", rec.answer.chars().take(240).collect::<String>());
    }
    // `HARNESS_EVAL_JUDGE=<model>`: ask a second model whether a failing
    // answer states something its tool results do not support.
    //
    // An advisory, and fenced like one. It is printed beside the failure
    // and counted at the end, and it never touches a verdict or a cause:
    // the rule that made this suite judge-free — a model grading a model
    // makes the grader's reliability the thing in question — still holds
    // for the board. What this adds is the half `UngroundedClaim` cannot
    // reach, a claim that is *wrong in meaning* rather than an identifier
    // nothing returned.
    //
    // It runs through `model_router` with the app's own `local_only`, so
    // the local/cloud switch governs it: in local-only mode a cloud judge
    // is refused exactly as any other cloud call would be.
    if o.verdict == Verdict::Fail {
        if let Ok(jm) = std::env::var("HARNESS_EVAL_JUDGE") {
            let local_only = settings["local_only"].as_bool().unwrap_or(false);
            let prompt = task.prompt.replace(MARKER_SLOT, &marker);
            match judge(&jm, local_only, &prompt, &rec).await {
                Ok(v) => {
                    if v.to_uppercase().starts_with("UNSUPPORTED") {
                        *judged_unsupported += 1;
                    }
                    *judged += 1;
                    eprintln!("      judge ({jm}): {v}");
                }
                Err(why) => eprintln!("      judge ({jm}): no verdict — {why}"),
            }
        }
    }
}
