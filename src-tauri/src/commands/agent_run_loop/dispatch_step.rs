use super::super::*;

mod screen;
use screen::*;

pub(in crate::commands) async fn dispatch_step(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    skills: &crate::core::skills::SkillStore,
    mcp: &mut McpBundle,
    http: &crate::core::egress::Client,
    depth: u8,
    specs: &[crate::core::agent::ToolSpec],
    // `(id, name, arguments, unreadable)` — normalised by the caller, because the two
    // wire formats name these differently and neither shape belongs in here.
    //
    // `unreadable` is `Some(why)` when the arguments never parsed. It travels with the
    // call rather than being resolved at the call site because a result has to land in
    // this step's slot for that call: both APIs require every tool_use to be answered
    // by its tool_result, so a call that is simply dropped is a protocol error.
    calls: &[(String, String, serde_json::Value, Option<String>)],
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    sink: &Option<EventSink>,
    shape: &crate::core::loopshape::LoopShape,
    // What the tier held back. `find_tools` searches this, and anything it finds is
    // returned to the caller to add to the wire for the rest of the turn.
    held: &[crate::core::agent::ToolSpec],
    // Where a call whose arguments did not work may be asked again under a grammar.
    // `None` on every wire that does not speak `format` — which is the gate, and the
    // reason a provider cannot reach that path by accident. See `core::argrepair`.
    repair: Option<&crate::core::argrepair::Repair>,
    // The turn's outstanding guess (HARNESS-RESEARCH-2026 §5). Lives across steps
    // because that is the whole point: the prefetch is started when a grep answers
    // and collected when the model, one generation later, asks for the file.
    speculator: &mut crate::core::speculation::Speculator,
) -> (Vec<(String, String, String)>, StepTiming, Vec<crate::core::agent::ToolSpec>) {
    use futures_util::StreamExt;
    use std::sync::atomic::Ordering;

    let began = std::time::Instant::now();
    let mut spent = std::time::Duration::ZERO;
    // Slots, so a result lands where its call was regardless of when it arrived.
    let mut slots: Vec<Option<String>> = vec![None; calls.len()];
    let mut revealed: Vec<crate::core::agent::ToolSpec> = Vec::new();

    // A guess that came good. Filled before execution, so a hit is a step that never
    // touches the disk — which is the latency the pattern was mined for. Done here
    // rather than inside the concurrent group because the speculator is borrowed
    // mutably and a `&mut` held across those awaits makes the whole turn future
    // non-Send, which fails to compile at every caller that spawns a turn.
    for (i, (_, name, input, unreadable)) in calls.iter().enumerate() {
        if unreadable.is_some() {
            continue;
        }
        if let Some(path) = input["path"].as_str() {
            if let Some(hit) = speculator.take(name, path) {
                emit_tool_started(app, sink, name, input);
                emit_tool_finished(app, sink, name, &hit);
                slots[i] = Some(hit);
            }
        }
    }

    // ---- A second chance at arguments that would not have worked — see `repair_calls`.
    // Owned from here on, because a repaired call carries different arguments than the
    // one that arrived and everything below reads them.
    let mut calls: Vec<(String, String, serde_json::Value, Option<String>)> = calls.to_vec();
    repair_calls(&mut calls, repair, specs, http, exec, cancel).await;
    let calls = &calls[..];

    // ---- Before anything is dispatched — see `screen_calls`.
    screen_calls(app, sink, calls, specs, held, &mut slots, &mut revealed);

    // Gated on what the model said it can do, not on what the tools allow. Phase 2
    // measures this: Ollama advertises `tools` and has no word for how many at once,
    // so a step whose second call is silently dropped is the failure mode.
    let together: Vec<usize> = (0..calls.len())
        .filter(|&i| slots[i].is_none() && shape.parallel_tools && parallel_safe(&calls[i].1, specs, mcp))
        .collect();

    // Concurrency is bounded by chunking rather than a semaphore or `buffered`: the
    // futures go into a plain `Vec` and `join_all` drives them. Iterator adapters over
    // async blocks that borrow `ToolCtx<'_>` defeat lifetime inference here — the
    // resulting future stops being `Send` and every caller that spawns a turn fails to
    // compile, with an error pointing at the spawn rather than at the cause.
    for chunk in together.chunks(PARALLEL_READS) {
        let mut futs = Vec::with_capacity(chunk.len());
        for &i in chunk {
            let (_, name, input, _) = &calls[i];
            futs.push(async move {
                if cancel.load(Ordering::SeqCst) {
                    return (i, None, std::time::Duration::ZERO);
                }
                emit_tool_started(app, sink, name, input);
                let t = std::time::Instant::now();
                let out = run_tool_no_mcp(app, state, exec, skills, http, depth, name, input).await;
                emit_tool_finished(app, sink, name, &out);
                (i, Some(out), t.elapsed())
            });
        }
        for (i, out, took) in futures_util::future::join_all(futs).await {
            spent += took;
            slots[i] = out;
        }
    }

    for (i, (_, name, input, _)) in calls.iter().enumerate() {
        if slots[i].is_some() {
            continue; // already run, above
        }
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        emit_tool_started(app, sink, name, input);
        let t = std::time::Instant::now();
        // C4. A plan is expanded here, in the loop, and not inside a tool — so every
        // step takes `run_tool`, the very path this call would have taken. The first
        // version expanded it inside `run_tool_inner`, which then recursed into
        // `run_tool_with`; the compiler could not prove that recursive future `Send`,
        // and every caller up to the HTTP handlers stopped compiling. Nothing below
        // `run_tool` calls back up to here, so there is no cycle to prove anything
        // through — and MCP tools can be plan steps too, under their own approval.
        let out = if name.as_str() == "run_plan" {
            run_plan_steps(app, state, exec, skills, mcp, http, depth, input).await
        } else {
            run_tool(app, state, exec, skills, mcp, http, depth, name, input).await
        };
        emit_tool_finished(app, sink, name, &out);
        spent += t.elapsed();
        slots[i] = Some(out);
    }

    // A write anywhere in this step makes any outstanding guess stale, and serving
    // pre-write contents would be a wrong answer produced by an optimisation — the
    // one outcome that would make this feature not worth having.
    if calls.iter().any(|(_, n, _, _)| matches!(n.as_str(), "write_file" | "edit_file" | "run_shell")) {
        speculator.invalidate();
    }
    // Then look for the next one. `consider` re-checks the policy itself — taint,
    // shape, size, confinement — so this call site does not restate it.
    for ((_, name, _, _), out) in calls.iter().zip(slots.iter()) {
        if let Some(text) = out {
            speculator.consider(&exec.root, name, text, exec.is_tainted());
        }
    }

    let timing = StepTiming { wall: began.elapsed(), summed: spent, parallel: together.len() };
    let elapsed = began.elapsed();
    crate::trace!(
        "turn",
        "step tools={} parallel={} elapsed={:.2}s summed={:.2}s saved={:.2}s",
        calls.len(),
        together.len(),
        elapsed.as_secs_f64(),
        spent.as_secs_f64(),
        (spent.as_secs_f64() - elapsed.as_secs_f64()).max(0.0)
    );

    let results = calls
        .iter()
        .zip(slots)
        .filter_map(|((id, name, _, _), out)| {
            // The full result is what the UI just showed; the transcript gets a
            // pointer when it is large enough to be worth a file.
            let out = offloaded(&exec.root, name, out?);
            Some((id.clone(), name.clone(), out))
        })
        .collect();
    (results, timing, revealed)
}
