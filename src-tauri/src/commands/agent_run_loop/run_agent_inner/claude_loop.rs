use super::*;
use serde_json::json;
use std::sync::atomic::Ordering;

/// Claude native tool use.
pub(super) async fn claude_loop(t: &mut Turn<'_>, history_msgs: Vec<serde_json::Value>) -> Result<Ran, String> {
    let Turn {
        app, state, sink, exec, persona, message, sid, model, api_model, depth, settings, profile, shape, http,
        specs, held, skills, mcp, cancel, budget, max_steps, sys_prompt, speculator, compaction_watch, tally, ..
    } = t;
    let (app, state, depth, max_steps) = (*app, *state, *depth, *max_steps);
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    let key = secrets::get_cloud_key().ok_or("This assistant is set to a Claude model — add an API key in Settings, or pick a local model.")?;
    let mut tools = tools_claude(&specs);
    tools.extend(mcp.claude_tools.iter().cloned());
    let mut messages = history_msgs;
    messages.push(json!({"role":"user","content":message}));
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
        // Same boundary as the OpenAI-compatible loop above — see the note there.
        for steer in steers(&sid) {
            emit("text", format!("Taking that into account: {steer}"));
            messages.push(json!({"role":"user","content": steer}));
        }
        // The step budget, said out loud — see the note in the branch above.
        let spent = tally.steps_taken.saturating_sub(1) as usize;
        if let Some(notice) = crate::core::loopshape::step_notice(spent, max_steps) {
            messages.push(json!({"role":"user","content": notice}));
        }
        let protect = crate::core::context::last_step_start(&messages);
        let obs = crate::core::observations::ObservationStore::for_session(&state.data_dir, &sid);
        crate::core::context::relieve_pressure(&mut messages, protect, budget.max_tokens, Some(&obs));
        let mut body = json!({"model":api_model,"max_tokens":settings.max_tokens,"messages":messages});
        // No tools on the final step, so the notice above is a fact rather than a
        // hint. Anthropic accepts a request with no `tools` key at all.
        if !crate::core::loopshape::is_final_step(spent, max_steps) {
            body["tools"] = json!(tools);
        }
        let reply = claude_step(model, settings, sys_prompt, &body, &messages, &key, cancel, state, sid, persona, api_model).await?;
        let Some(reply) = reply else {
            emit("stopped", "Stopped.".into());
            answer = "Stopped.".into();
            break;
        };
        if let Some(u) = &reply.usage {
            let acc = turn_usage.get_or_insert_with(Default::default);
            acc.input += u.input;
            acc.output += u.output;
            acc.cache_read += u.cache_read;
            acc.cache_write += u.cache_write;
            acc.model = u.model.clone();
            // Per STEP, matching the OpenAI branch: the status line under the
            // transcript should climb while the work happens.
            emit("usage", acc.total().to_string());
        }
        let content = reply.content.clone();
        let stop = reply.stop_reason.as_str();

        let mut tool_uses = Vec::new();
        for b in &content {
            match b["type"].as_str() {
                Some("text") => { let t = b["text"].as_str().unwrap_or("").trim().to_string(); if !t.is_empty() { emit("text", t); } }
                Some("tool_use") => tool_uses.push(b.clone()),
                _ => {}
            }
        }
        tools_ran |= !tool_uses.is_empty();
        messages.push(json!({"role":"assistant","content":content.clone()}));
        if tool_uses.is_empty() || stop == "end_turn" {
            let ft = content.iter().filter_map(|b| if b["type"] == "text" { b["text"].as_str() } else { None }).collect::<Vec<_>>().join("\n");
            if ft.trim().is_empty() {
                crate::trace!("turn", "empty step — no blocks of text, no tool use");
                // Same nudge as the branch above, for the same reason.
                if !nudged_empty && !crate::core::loopshape::is_final_step(spent, max_steps) {
                    nudged_empty = true;
                    messages.push(json!({
                        "role": "user",
                        "content": crate::core::loopshape::EMPTY_REPLY_NUDGE
                    }));
                    continue;
                }
            }
            answer = if ft.trim().is_empty() {
                crate::core::loopshape::empty_answer(tools_ran)
            } else {
                ft
            };
            // Same gate as the loop above — see `citation_challenge`.
            if let Some(rebuke) = citation_challenge(&answer, &exec.citable(), &exec.prompt, &mut challenged) {
                emit("text", "Checking the links in that answer…".into());
                messages.push(json!({"role":"user","content": rebuke}));
                continue;
            }
            break;
        }
        // Reads overlap; everything else stays in the model's order.
        //
        // Claude routinely emits three or four `tool_use` blocks in one message
        // precisely so they can run at once, and this ran them one after another —
        // three 300ms reads costing 900ms instead of 300. `parallel_safe` says
        // which may share a moment; `RiskClass::Read` is the marker, and the
        // approval prompts, taint propagation and coverage ledger keep the
        // sequential order they were written against.
        let calls: Vec<(String, String, serde_json::Value, Option<String>)> = tool_uses
            .iter()
            .map(|tu| {
                (
                    tu["id"].as_str().unwrap_or("").to_string(),
                    tu["name"].as_str().unwrap_or("").to_string(),
                    tu["input"].clone(),
                    // This wire format delivers `input` as an object the server
                    // already parsed, so there is no string here to fail on.
                    None,
                )
            })
            .collect();
        let (done, timing, revealed) = dispatch_step(
            app, state, &exec, &skills, mcp, &http, depth, &specs, &calls, &cancel, &sink,
            &shape, &held, None, speculator,
        )
        .await;
        tally.tool_wall += timing.wall;
        tally.tool_summed += timing.summed;
        tally.tools_parallel += timing.parallel as u32;
        // Same reveal, the other wire format. Sorted in, not appended: `tools` is
        // first in the cache hierarchy and Phase 3 keeps it ordered by name.
        for t in &revealed {
            tools.push(json!({"name":t.name,"description":t.description,"input_schema":t.schema}));
        }
        tools.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
        specs.extend(revealed);
        let results: Vec<serde_json::Value> = done
            .into_iter()
            .map(|(id, _, out)| json!({"type":"tool_result","tool_use_id":id,"content":out}))
            .collect();
        // ALL of them in one user message. Splitting them across messages trains
        // the model out of asking for several at a time, which is the behaviour
        // this whole change exists to exploit.
        messages.push(json!({"role":"user","content":results}));
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
