use super::super::*;

/// Execute one tool call. Tools needing app-level context (skills, subagents,
/// computer use, connected-app MCP tools) are resolved here; everything else goes
/// to the dispatcher.
#[allow(clippy::too_many_arguments)]
/// One tool call, with every gate in front of it. `mcp` is `None` on the parallel
/// path — see `run_tool_inner`.
#[allow(clippy::too_many_arguments)]
pub(in crate::commands) async fn run_tool_with(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    skills: &crate::core::skills::SkillStore,
    mcp: Option<&mut McpBundle>,
    http: &crate::core::egress::Client,
    depth: u8,
    name: &str,
    args: &serde_json::Value,
) -> String {
    // Gates first — before the permission check, before MCP, before anything runs.
    //
    // The order matters. A call with an invented path is not a call to approve or
    // deny, it is a call that was never about a real thing, and asking the user to
    // approve reading `path/to/sales.xlsx` would be asking them to rubber-stamp a
    // hallucination. See `core::gates` for what each one refuses and why it can only
    // refuse things that cannot be true.
    let signature = crate::core::gates::signature(name, args);
    // The stall gate stands apart from the others, and deliberately does NOT record a
    // failure. Everything below is a call that could not work; a stalled call works
    // perfectly and simply has nothing left to say. Counting it as a failure would let
    // the repetition gate take over after two refusals and tell the model "it failed
    // every time" about a call that has succeeded every time — a false sentence in a
    // message whose whole job is to be read closely and acted on.
    if let Some(refusal) = crate::core::gates::stalled(name, exec.stalled_count(&signature)) {
        // Traced because the first attempt to verify this gate could not answer
        // "did it fire?" at all. A refusal is returned to the model as a tool result;
        // it never reaches the trace log, and the eval path writes no run journal —
        // so the only honest answer after a 300-second run was "no idea". A gate whose
        // firing cannot be observed cannot be evidence for or against itself.
        crate::trace!("turn", "stall gate refused {name} after {} identical results", exec.stalled_count(&signature));
        return refusal;
    }
    if let Some(refusal) = crate::core::gates::repetition(name, exec.failure_count(&signature))
        // The hunt that `repetition` cannot see: every guess has different arguments,
        // so every one is a fresh signature. Counted across the run instead, and
        // cleared by any call that actually looks.
        .or_else(|| crate::core::gates::groping(name, exec.grope_count()))
        .or_else(|| crate::core::gates::placeholder(name, args, &exec.prompt))
        // Charts, tables and cards are the outputs read as measurement rather than
        // as opinion, so they are the ones that have to name a source and have it
        // checked. A table of invented figures is the same lie as a chart of them —
        // arguably a plainer one, since it shows the numbers themselves — and a card
        // is the most trusted of the three, being the one that says what to conclude.
        .or_else(|| {
            matches!(name, "chart" | "table" | "card")
                .then(|| {
                    crate::core::gates::grounding(args["source"].as_str().unwrap_or(""), &exec.seen_list())
                })
                .flatten()
        })
    {
        exec.note_failure(&signature);
        return refusal;
    }
    // The one door — `core::gateway`. Here, after the gates above (a call about a thing
    // that was never real has nothing to decide and nothing to record) and before
    // anything runs, so there is no path from a tool call to an action that does not
    // pass the user's boundaries and write its row first.
    let governed = !crate::core::gateway::UNGOVERNED.contains(&name);
    let ticket = if governed {
        let mcp_call = mcp.as_ref().and_then(|m| {
            m.registry.get(name).map(|(idx, raw)| {
                let risk = crate::core::overrides::RiskOverrides::load(&exec.data_dir).effective(name, mcp_risk(raw));
                crate::core::gateway::McpCall {
                    server: m.names.get(*idx).cloned().unwrap_or_else(|| name.split('_').next().unwrap_or("app").to_string()),
                    tool: raw.clone(),
                    effect: if risk == crate::core::agent::RiskClass::Read { "read" } else { "write" }.to_string(),
                }
            })
        });
        let subject = crate::core::gateway::with_browser_element(
            crate::core::gateway::subject_for(name, args, mcp_call.as_ref(), exec.is_tainted()),
            &exec.persona,
        );
        match crate::core::gateway::before(&exec.data_dir, &exec.gateway_facts(), subject) {
            Ok(t) => Some(t),
            // Not counted as a failure: the repetition gate's "it failed every time" would
            // be a false sentence about a call that never ran, and the refusal already
            // says the one thing that matters — stop.
            Err(refusal) => return refusal,
        }
    } else {
        None
    };
    let out = run_tool_inner(app, state, exec, skills, mcp, http, depth, name, args).await;
    if let Some(t) = &ticket {
        let stopped = session_cancel_flag(&exec.session_id).load(std::sync::atomic::Ordering::SeqCst);
        crate::core::gateway::after(&exec.data_dir, &exec.gateway_facts(), t, &out, stopped);
    }
    // Remember how it went, so the repetition gate can tell a retry from a loop and
    // the grounding gate knows what this run has actually read.
    if tool_status(&out) == "error" {
        exec.note_failure(&signature);
    } else {
        exec.note_success(&signature);
        // ...and whether it actually told us anything. A call that succeeds is not
        // the same as a call that progresses, and only this can tell them apart.
        exec.note_locator(name, &out);
        let identical = exec.note_result(&signature, &out);
        // The near-miss, traced. Whether a repeat happens *at all* on real turns is
        // the question behind "is this gate worth having", and a log that only records
        // the refusal answers it just once — at the threshold, after the behaviour is
        // already established. This records the run-up.
        if identical > 1 {
            crate::trace!("turn", "{name} returned an identical result ({identical} in a row)");
        }
        if let Some(read) = read_target(name, args) {
            exec.note_seen(&read);
        }
        // A read of whatever page the browser is on — after a click, not the address the
        // run asked for — so it is the browser that knows what was read.
        if name == "browser_read" {
            if let Some(url) = crate::core::agentbrowser::current_url(&exec.persona) {
                exec.note_seen(&url);
            }
        }
        // Keep the visual itself, not just the file it wrote — see ToolCtx::charts. All
        // four are specs the transcript redraws; keeping only the chart left every other
        // one to vanish on reload, under an answer that still referred to it.
        if matches!(name, "chart" | "table" | "card" | "ui_component") {
            exec.note_visual(name, args);
        }
        // Links this run genuinely came across: search hits, links on a page it
        // fetched, URLs in a connected app's reply. The citation gate treats these as
        // real, because they are — a link returned by a real search exists, even if
        // the run never opened it. Bounded per call so one link-heavy page cannot
        // crowd out everything else the run has seen.
        for u in crate::core::gates::urls_in(&out).into_iter().take(24) {
            exec.note_url(&u);
        }
    }
    out
}

/// Which of a step's calls may run at the same time as each other.
///
/// `RiskClass::Read` is the marker, and it is not a guess: a Read tool returns content
/// and changes nothing, so two of them overlapping cannot produce a different result
/// than running them in order. Everything else — `WriteLocal`, `Exec`, `External` —
/// stays serial in the model's emitted order, which is what the approval prompts, the
/// taint propagation and the coverage ledger are all written against.
///
/// MCP tools are excluded whatever their risk class. Calling one needs `&mut
/// McpBundle` for its session state, and a risk class the user can override in
/// Settings is not a claim about thread-safety.
pub(in crate::commands) fn parallel_safe(name: &str, specs: &[crate::core::agent::ToolSpec], mcp: &McpBundle) -> bool {
    if mcp.registry.contains_key(name) {
        return false;
    }
    // The browser is one page shared by every call, and a snapshot is not a pure read:
    // it retires the refs of the one before. Two overlapping calls on it can see
    // different pages, so its reads keep their order too.
    if crate::core::agent::is_browser_tool(name) {
        return false;
    }
    specs
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.risk == crate::core::agent::RiskClass::Read)
        .unwrap_or(false)
}

/// How many reads may be in flight at once.
///
/// Small on purpose. These are local file reads and web fetches; the win is turning a
/// sum into a max, and the difference between 4 and 40 is not that win, it is a
/// thundering herd against someone's disk or a site's rate limit.
pub(in crate::commands) const PARALLEL_READS: usize = 4;

/// A tool call starting. A free function rather than a closure so the step dispatcher
/// can emit without taking one as a parameter: a closure with reference parameters is
/// not general enough over lifetimes to be held across the awaits in the concurrent
/// group, and a `&dyn Fn` is not `Sync`, which makes the whole turn future non-Send.
pub(in crate::commands) fn emit_tool_started(app: &tauri::AppHandle, sink: &Option<EventSink>, name: &str, args: &serde_json::Value) {
    use serde_json::json;
    use tauri::Emitter;
    let payload = json!({ "type": "tool", "name": name, "arguments": args, "text": tool_label(name, args) });
    let _ = app.emit("agent-run", payload.clone());
    if let Some(tx) = sink {
        let _ = tx.send(payload);
    }
}
