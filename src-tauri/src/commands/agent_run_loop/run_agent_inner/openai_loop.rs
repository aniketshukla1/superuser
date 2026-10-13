use super::*;
use serde_json::json;
use std::sync::atomic::Ordering;

/// OpenAI-compatible tool-calling (Ollama + OpenAI-style cloud providers).
pub(super) async fn openai_loop(t: &mut Turn<'_>, history_msgs: Vec<serde_json::Value>) -> Result<Ran, String> {
    let mut w = openai_wire(t)?;
    let Turn {
        app, state, sink, exec, persona, message, sid, model, api_model, depth, settings, profile, shape, http,
        specs, held, skills, mcp, cancel, budget, max_steps, sys_prompt, speculator, compaction_watch, tally, ..
    } = t;
    let (app, state, depth, max_steps) = (*app, *state, *depth, *max_steps);
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    let mut messages = vec![json!({"role":"system","content": sys_prompt.joined()})];
    messages.extend(history_msgs);
    messages.push(json!({"role":"user","content":message}));
    // Everything from here on is this turn's own traffic. The system prompt, the
    // replayed history and the user's message are all recorded elsewhere; storing
    // them again would grow every session file by the whole conversation.
    let turn_start = messages.len();
    let mut answer = "Reached the step limit.".to_string();
    // One citation challenge per turn — see `citation_challenge`.
    let mut challenged = false;
    // One nudge per turn when the model replies with nothing, and whether any tool
    // ever ran — which decides whether "Done." is a fair summary or a lie.
    let mut nudged_empty = false;
    let mut tools_ran = false;
    let mut turn_usage: Option<crate::core::sessions::TokenUsage> = None;
    for _ in 0..max_steps {
        tally.steps_taken += 1;
        if cancel.load(Ordering::SeqCst) { emit("stopped", "Stopped.".into()); answer = "Stopped.".into(); break; }
        // Anything the user typed while this turn was working. Here is the safe
        // point: the previous step's tool results are already committed, so the
        // tool_use/tool_result pairing both APIs require is intact.
        for steer in steers(&sid) {
            emit("text", format!("Taking that into account: {steer}"));
            messages.push(json!({"role":"user","content": steer}));
        }
        // The step budget, said out loud. Injected here for the same reason steers
        // are: the previous step's tool results are committed, so the pairing both
        // APIs require is intact.
        let spent = tally.steps_taken.saturating_sub(1) as usize;
        if let Some(notice) = crate::core::loopshape::step_notice(spent, max_steps) {
            messages.push(json!({"role":"user","content": notice}));
        }
        // Shrink tool output from earlier steps in this turn. The step just
        // executed is left whole — that's the one the model is reasoning about.
        // The cut text goes to the session's observation store first, so the
        // notice can name a handle the model can call `recall` on instead of
        // telling it to re-read a source that may not exist any more.
        let protect = crate::core::context::last_step_start(&messages);
        let obs = crate::core::observations::ObservationStore::for_session(&state.data_dir, &sid);
        crate::core::context::relieve_pressure(&mut messages, protect, budget.max_tokens, Some(&obs));
        let body = step_body(&w, api_model, max_steps, profile, settings, model, &messages, spent);
        let Some(resp) = send_step(&w, http, state, cancel, sid, persona, api_model, body).await? else {
            emit("stopped", "Stopped.".into());
            answer = "Stopped.".into();
            break;
        };
        let streamed =
            stream_chat_message(resp, w.openai, &cancel, w.silence, |t| emit("text", t), |t| emit("thinking", t)).await?;
        // The model went quiet mid-answer. The turn ends with a sentence rather than
        // with whatever half of a reply had arrived, which the loop would otherwise
        // read as an answer — or, worse, as a tool call to run.
        if let Some(silent) = streamed.stalled {
            return Err(crate::core::stall::end_turn(&state.data_dir, &sid, &persona, &api_model, silent, w.local_model));
        }
        if let Some(u) = streamed.usage {
            let acc = turn_usage.get_or_insert_with(Default::default);
            acc.input += u.input;
            acc.output += u.output;
            // Per STEP, not per turn: a turn is several model calls and the status line
            // under the transcript should climb while the work happens, not jump to its
            // final figure once there is nothing left to wait for.
            emit("usage", acc.total().to_string());
        }
        if streamed.stopped {
            emit("stopped", "Stopped.".into());
            answer = "Stopped.".into();
            break;
        }
        let mut msg = streamed.message;
        let calls = salvage_printed(&mut msg, specs);
        tools_ran |= !calls.is_empty();
        messages.push(msg.clone());
        if calls.is_empty() {
            let text = msg["content"].as_str().unwrap_or("").trim().to_string();
            // A step with no tool calls AND no text becomes the answer "Done.",
            // which is indistinguishable from a model that finished politely.
            // The scoreboard found three tasks failing this way on its first run,
            // and telling "the model said nothing" from "we lost what it said"
            // took an hour of bisecting against raw curl. This is that hour,
            // spent once.
            if text.is_empty() {
                crate::trace!(
                    "turn",
                    "empty step — no calls, no text. raw message: {}",
                    crate::core::trace::preview(&msg.to_string())
                );
                // Ask once. A model that lost its footing usually recovers when
                // told; one that does not will not recover on the tenth attempt
                // either, and each try is another wait. Skipped on the final step,
                // where there is no step left to answer in.
                if !nudged_empty && !crate::core::loopshape::is_final_step(spent, max_steps) {
                    nudged_empty = true;
                    messages.push(json!({
                        "role": "user",
                        "content": crate::core::loopshape::EMPTY_REPLY_NUDGE
                    }));
                    continue;
                }
            }
            answer = if text.is_empty() {
                crate::core::loopshape::empty_answer(tools_ran)
            } else {
                text
            };
            // The last gate, and the only one that reads prose. An answer is
            // where a fabricated source finally reaches the user, so it is
            // checked before it gets there rather than after.
            if let Some(rebuke) = citation_challenge(&answer, &exec.citable(), &exec.prompt, &mut challenged) {
                emit("text", "Checking the links in that answer…".into());
                messages.push(json!({"role":"user","content": rebuke}));
                continue;
            }
            break;
        }
        // Same dispatcher as the Claude branch — reads together, everything else
        // in the model's order. Stop still means stop: it is checked before each
        // call rather than only at the top of the step, so a model that asked for
        // four tools does not run all four after the user pressed it.
        let normalised: Vec<(String, String, serde_json::Value, Option<String>)> = calls
            .iter()
            .map(|c| {
                let (args, unreadable) = read_arguments(&c["function"]["arguments"]);
                (
                    c["id"].as_str().unwrap_or("").to_string(),
                    c["function"]["name"].as_str().unwrap_or("").to_string(),
                    args,
                    unreadable,
                )
            })
            .collect();
        let (done, timing, revealed) = dispatch_step(
            app, state, &exec, &skills, mcp, &http, depth, &specs, &normalised, &cancel, &sink,
            &shape, &held, w.repair.as_ref(), speculator,
        )
        .await;
        tally.tool_wall += timing.wall;
        tally.tool_summed += timing.summed;
        tally.tools_parallel += timing.parallel as u32;
        // A withheld tool the model just asked for, added to the wire for the rest
        // of the turn. The format differs per branch, which is the one thing that
        // cannot live in the dispatcher.
        for t in &revealed {
            w.tools.push(json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.schema}}));
        }
        specs.extend(revealed);
        for (id, _, out) in done {
            let mut tmsg = json!({"role":"tool","content":out});
            if w.openai {
                tmsg["tool_call_id"] = json!(id);
            }
            messages.push(tmsg);
        }
    }
    if let Some(w) = compaction_watch.as_mut() {
        w.observe_turn(&messages);
        // The macro, not `trace::enabled` directly: that function is
        // `#[cfg(debug_assertions)]` and calling it names something a release
        // build does not have. The macro expands to nothing there instead.
        crate::trace!("compaction", "{}", w.summary());
    }
    Ok(Ran {
        answer,
        usage: turn_usage,
        wire: crate::core::transcript::TurnWire::capture(
            crate::core::transcript::WireShape::of(profile),
            model,
            messages.get(turn_start..).unwrap_or_default(),
        ),
    })
}
