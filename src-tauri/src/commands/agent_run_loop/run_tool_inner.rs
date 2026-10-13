use super::super::*;

#[allow(clippy::too_many_arguments)]
/// `mcp` is optional because a tool that is not an MCP tool never touches it: every
/// use is inside the registry-hit block below, which returns. The parallel path
/// (`run_tool_no_mcp`) passes `None` — a `&mut McpBundle` cannot be shared across
/// concurrent calls, and pretending otherwise is how a refactor takes the exclusivity
/// the MCP session state depends on.
pub(in crate::commands) async fn run_tool_inner(
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
    // A connected app's tool. Gated first: an assistant running an unattended
    // automation must not be able to post, file or delete things in the user's
    // accounts just because an MCP server offered a tool for it.
    if let Some((mcp, (idx, raw))) =
        mcp.and_then(|m| m.registry.get(name).cloned().map(|hit| (m, hit)))
    {
        use crate::core::inbox::{decide, Ask, Decision, Inbox};
        // The name-based guess, then the user's correction of it. Loaded per call
        // rather than cached: the file is tiny, and an edit taking effect on the
        // next tool call is what someone tuning a noisy server actually wants.
        let risk = crate::core::overrides::RiskOverrides::load(&exec.data_dir).effective(name, mcp_risk(&raw));
        let target = name.split('_').next().unwrap_or("app").to_string();
        let summary = format!("Run {raw} on {target}");
        // A server the user marked "ask before acting" is never waved through, even
        // when the name-based risk guess says Read. The flag is the user's statement
        // about a server they chose to add; it may narrow the decision, never widen
        // it, so a Deny still stands.
        let mut decision = decide(name, &target, risk, &exec.mode, &exec.standing, exec.is_tainted());
        if decision == Decision::Allow && mcp.always_ask.contains(&idx) {
            decision = Decision::Ask;
        }
        match decision {
            Decision::Deny => {
                return format!("Not run — this assistant's mode ({}) doesn't allow acting in connected apps.", exec.mode)
            }
            Decision::Ask => {
                let ask = Inbox::load(&exec.data_dir).park(Ask::new(
                    &exec.session_id, exec.task_id.clone(), name, &target, &summary, args.clone(),
                ));
                return format!(
                    "Parked for approval (inbox id {}). Nothing was changed in {target}.{}",
                    ask.id,
                    if exec.unattended { "" } else { " Tell the user it's waiting for their approval." }
                );
            }
            Decision::Allow => {}
        }
        // What is about to leave, checked before it does — Phase 6. The model has spent
        // the turn reading files, pages and replies, and a credential it read on the way
        // past cannot be recalled once it is in somebody's issue tracker.
        let findings = crate::core::contentgov::inspect(args);
        if !findings.is_empty() {
            let said = crate::core::contentgov::refusal(&target, &findings);
            crate::core::audit::log(
                &exec.data_dir,
                "refuse",
                &exec.session_id,
                &target,
                // The trail gets the same thing the model gets: kinds and paths, never values.
                &format!("held back a call carrying {}", findings.iter().map(|f| f.say()).collect::<Vec<_>>().join("; ")),
                false,
            );
            return said;
        }
        // Recorded by the gateway before this point, as `write_tool` or `read_tool`.
        return match mcp.sessions[idx].call_tool(http, &raw, args.clone()).await {
            Ok(r) => {
                // Whatever the app returned was written by someone else — a page
                // body, an issue comment, a message. Same standing as a synced
                // memory or a fetched web page, so the run carries the same taint.
                exec.taint();
                // Three different things that must never read alike: the app saying no,
                // the app saying nothing, and the app answering. The middle one caused
                // real harm — a vendor's "the caller does not have permission" was read
                // back to a person as a fact about their own account.
                if r["isError"].as_bool().unwrap_or(false) {
                    return crate::core::contentgov::vendor_error(&mcp_result_text(&r));
                }
                let text = mcp_result_text(&r);
                if text.trim().is_empty() || text.trim() == "{}" {
                    return crate::core::contentgov::empty_result(&target, &raw);
                }
                text
            }
            Err(e) => {
                // Access taken away is not a call that went wrong: it will fail the same
                // way forever until somebody signs in again, and a run that retries it
                // looks broken for a reason nobody can see.
                if crate::core::contentgov::is_revoked(&e) {
                    crate::core::audit::log(
                        &exec.data_dir,
                        "connector",
                        &exec.session_id,
                        &target,
                        "the app stopped accepting this app's sign-in — it needs reconnecting",
                        false,
                    );
                    return crate::core::contentgov::revoked(&target);
                }
                crate::core::contentgov::call_failed(&e)
            }
        };
    }
    match name {
        "load_skill" => {
            let sk = args["name"].as_str().unwrap_or("");
            // The same skills the catalogue it was shown holds: switched on, and on this
            // assistant's own list when it has one. Loading by name used to reach past both.
            let allowed = crate::core::personas::PersonaRegistry::load(&exec.data_dir)
                .manifest(&exec.persona)
                .map(|m| m.skills.clone())
                .unwrap_or_default();
            match skills.usable(sk, &allowed) {
                Some(s) => s.instructions.clone(),
                None if skills.get(sk).is_none() => format!("unknown skill: {sk}"),
                None if !skills.is_on(sk) => format!("error: the skill `{sk}` is switched off. The person can switch it on in Settings ▸ Skills."),
                None => format!("error: `{sk}` isn't one of this assistant's skills."),
            }
        }
        "spawn_subagent" => run_subagents(app, state, exec, depth, args).await,
        "ask_coworker" => ask_coworker(app, state, exec, depth, args).await,
        "computer_use" => {
            let task = args["task"].as_str().unwrap_or("").trim();
            if task.is_empty() {
                return "error: describe what to do on screen".into();
            }
            // Desktop control is Exec-class: gate it like any consequential action.
            use crate::core::inbox::{decide, Decision};
            match decide("computer_use", "desktop", crate::core::agent::RiskClass::Exec, &exec.mode, &exec.standing, exec.is_tainted()) {
                Decision::Deny => format!("Not run — this assistant's mode ({}) doesn't allow driving the desktop.", exec.mode),
                Decision::Ask if exec.unattended => {
                    let ask = crate::core::inbox::Inbox::load(&exec.data_dir).park(crate::core::inbox::Ask::new(
                        &exec.session_id, exec.task_id.clone(), "computer_use", "desktop", &format!("Drive the desktop: {task}"), args.clone()));
                    format!("Parked for approval (inbox id {}) — the desktop was not touched.", ask.id)
                }
                _ => {
                    match cu_run(app.clone(), state, task.to_string(), None).await {
                        Ok(summary) => summary,
                        Err(e) => format!("computer use failed: {e}"),
                    }
                }
            }
        }
        _ => crate::core::agent_exec::dispatch(exec, name, args).await,
    }
}

/// Does this answer need sending back? Some(rebuke) when it cites links the run
/// never opened and the model has not already been asked about it once.
///
/// Once, and only once. The rest of the gates refuse a call that hasn't happened
/// yet, so repeating one costs a step. This one arrives after the work is done, so
/// a second round costs the user a whole extra model call on an answer they can
/// already read — and a model that re-cites the same link after being told is not
/// going to be argued round on the third pass. `challenged` is the latch.
///
/// Takes what it needs rather than the whole `ToolCtx` so the latch is testable:
/// a gate that can loop is exactly the kind you want a test standing on.
pub(in crate::commands) fn citation_challenge(
    answer: &str,
    citable: &[String],
    said: &str,
    challenged: &mut bool,
) -> Option<String> {
    if *challenged || answer.trim().is_empty() {
        return None;
    }
    let bad = crate::core::gates::uncited_urls(answer, citable, said);
    if bad.is_empty() {
        return None;
    }
    *challenged = true;
    Some(crate::core::gates::citation_rebuke(&bad))
}
