use super::*;

mod print_outcome;
use print_outcome::*;

    #[tokio::test]
    #[ignore = "the scoreboard: needs the app running and a model. See the module note."]
    pub(super) async fn scoreboard() {
        let Some(tok) = token() else {
            panic!("no sidecar token — is the app running?");
        };
        // Through the spine like everything else. The rule is not waived for a test
        // helper: `egress::client` is what makes traffic visible on the Privacy page
        // and what local-only mode acts on, and a scoreboard that quietly opened its
        // own socket would be the one piece of this app exempt from its own policy.
        let http = crate::core::egress::client("harness-eval");
        let settings = settings(&http, &tok).await.expect("app not reachable on :8765");
        let budget = settings["agent_max_steps"].as_u64().unwrap_or(24) as usize;

        let models: Vec<String> = std::env::var("HARNESS_EVAL_MODELS")
            .ok()
            .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_else(|| {
                // Whatever the app itself offers, so a run on someone else's machine
                // measures their models rather than failing on ids they do not have.
                settings["models"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).take(1).collect())
                    .unwrap_or_default()
            });
        assert!(!models.is_empty(), "no models to test; set HARNESS_EVAL_MODELS");
        // Fresh per run, so the only copy in existence is the one written into
        // the fixture. A literal in the source is findable by a shell that can
        // read the repository — which this one can, by design.
        let marker: String = format!("MK{:010X}", rand::random::<u64>() & 0xff_ffff_ffff);
        eprintln!(
            "scoreboard [{}]: {} tasks x {} models (marker {marker})",
            split_tag_name(),
            suite().len(),
            models.len()
        );

        // Repeats, because one run of one task is not a measurement — see
        // `Outcome::run`. Default 1 so a quick sweep stays quick; the number that goes
        // into HARNESS-BASELINE should come from more than one.
        let repeats: usize = std::env::var("HARNESS_EVAL_REPEATS")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|n| *n > 0)
            .unwrap_or(1);
        // §6a, said before the run rather than discovered after it. The power
        // analysis behind this: ~2-3 runs to detect a 5% effect, ~9 for 2%, ~36 for
        // 1%. Every item left in HARNESS-RESEARCH-2026 is plausibly a 2-4 point
        // change, which is to say invisible at three repeats. Printing it here is
        // cheaper than another round withdrawn for measuring nothing.
        eprintln!(
            "power: {repeats} repeat(s) resolves ~{} — {}",
            match repeats {
                0..=1 => "nothing; a single draw is not a measurement",
                2..=4 => "a 5-point move, nothing smaller",
                5..=8 => "roughly a 3-point move",
                _ => "roughly a 2-point move",
            },
            if repeats < 5 {
                "treat anything smaller as noise (HARNESS-RESEARCH-2026 §6a)"
            } else {
                "small moves are readable at this count"
            }
        );
        let mut outcomes: Vec<Outcome> = Vec::new();
        let mut tools = crate::core::toolstats::Tally::new();
        let (mut judged, mut judged_unsupported) = (0usize, 0usize);
        let mut abandoned = false;
        for run in 0..repeats {
          if abandoned {
              break;
          }
          if repeats > 1 {
              eprintln!("--- run {} of {repeats} ---", run + 1);
          }
          for model in &models {
            if abandoned {
                break;
            }
            for (i, task) in suite().iter().enumerate() {
                if abandoned {
                    break;
                }
                // `HARNESS_EVAL_TASKS=a,b` narrows the run. Re-checking one change on
                // `gemma4:12b` otherwise costs 83 minutes of tasks that change was
                // never going to touch, and a measurement that expensive is one that
                // quietly stops being taken.
                //
                // The session id still keys on the task's index in the suite, not on its
                // position in the filtered run, so a task's working folder is the same
                // whether it ran alone or in a full sweep.
                //
                // The split is part of the id because the two suites share an index
                // space: without it `eval-0-gemma4` would name a different task
                // depending on an environment variable, and the second run would
                // inherit the first one's working folder and its fixtures.
                if !task_selected(task.id) {
                    continue;
                }
                let sid = format!("eval-{}{}-{}", split_prefix(), i, sanitise(model));
                let mut rec = run_one(&http, &tok, &settings, model, task, &sid, &marker).await;
                // Unreachable means the app went away, not that the model failed.
                // Give it a chance to come back before writing off the rest of a
                // sweep that may have hours left in it.
                if rec.unreachable.is_some() {
                    eprintln!("  ....  app unreachable — waiting up to 3 minutes for it");
                    if wait_for_app(&http, &tok, 180).await {
                        eprintln!("  ....  app is back; retrying {}", task.id);
                        rec = run_one(&http, &tok, &settings, model, task, &sid, &marker).await;
                    } else {
                        // Loudly, and once. A run that carries on against a dead port
                        // produces a table of blanks that reads like a result.
                        eprintln!(
                            "\n!! the app never came back. ABANDONING the run after \
                             {} scored outcome(s) — everything below is unmeasured, and \
                             this sweep is void rather than bad.\n",
                            outcomes.iter().filter(|o| o.verdict != Verdict::NoResult).count()
                        );
                        abandoned = true;
                    }
                }
                let mut o = score(task, model, &rec, budget);
                o.run = run;
                // TOOLS-RESEARCH-2026 §11. Every record already carries the calls;
                // nothing ever added them up. Folded here rather than in `score`,
                // which is a pure function over one record and stays that way.
                for c in &rec.calls {
                    tools.record(&c.name, crate::core::toolstats::is_failure(&c.result), task.id);
                }
                print_outcome(&o, task, model, &rec, &settings, &marker, &mut judged, &mut judged_unsupported).await;
                outcomes.push(o);
            }
          }
        }
        eprintln!("\n{}", report(&outcomes));

        // The per-tool fold. Under the task table on purpose: it says what these
        // tasks exercise, which is a fact about the tasks before it is a fact about
        // the catalogue. `toolstats::real_usage` is the other half and the one that
        // can say a tool is unused.
        {
            let catalogue = crate::core::agent::every_tool();
            let resident: Vec<&str> = catalogue
                .iter()
                .map(|(_, n)| n.as_str())
                .filter(|n| crate::core::agent::is_resident(n))
                .collect();
            eprintln!("{}", tools.render("Tools, under this sweep", &catalogue, &resident));
        }
        if judged > 0 {
            eprintln!(
                "\njudge: {judged_unsupported} of {judged} failing answers read as stating something their tool \
                 results do not support. Advisory — it changes no verdict and no cause."
            );
        }

        // §5's gate: the column has existed for a while and nothing asserted on it.
        // Reported rather than fatal, because the honest tolerance is wide — this
        // machine has been measured running a build beside a sweep — and a gate that
        // fails on the laptop is a gate people learn to ignore.
        let previous = history();
        if let Some(prev) = &previous {
            let now = History::from_outcomes(&marker, &outcomes);
            let slower = now.slower_than(prev);
            if slower.is_empty() {
                eprintln!("seconds: no task is more than 50% slower than {}", prev.measured_at);
            } else {
                eprintln!("seconds: {} task(s) materially slower than {}", slower.len(), prev.measured_at);
                for (id, was, now) in slower {
                    eprintln!("  {id}: {was:.0}s -> {now:.0}s ({:.1}x)", now / was);
                }
            }
        }

        // Record what this sweep measured so a later one can narrow to the band.
        // Written unconditionally: a band run is only as good as the history behind
        // it, and a history nobody updates is the stale number round 1 warned about.
        let hist = History::from_outcomes(&marker, &outcomes);
        match serde_json::to_string_pretty(&hist) {
            Ok(j) => match std::fs::write(history_path(), j) {
                Ok(()) => eprintln!("history: {} task rates -> {}", hist.rates.len(), history_path().display()),
                Err(e) => eprintln!("history: not written ({e})"),
            },
            Err(e) => eprintln!("history: not serialised ({e})"),
        }
    }
