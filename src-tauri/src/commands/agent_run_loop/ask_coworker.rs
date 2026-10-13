use super::super::*;

/// Hand a piece of work to another assistant, and relay its answer back — D3.
///
/// Synchronous, with a deadline, because a person is waiting in the conversation that
/// asked. The asked assistant runs as itself in a conversation of its own that the person
/// can open afterwards, which is where its answer lives; what comes back here is that
/// answer, attributed, for the asking run to use in its own.
pub(in crate::commands) async fn ask_coworker(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    depth: u8,
    args: &serde_json::Value,
) -> String {
    use crate::core::coworker;
    use std::sync::atomic::Ordering;

    // A subagent is one turn's work broken up, not a colleague with a roster of its own.
    if depth > 0 {
        return "A subagent can't hand work on to another assistant. Do this step yourself, or say what is missing.".into();
    }
    let note = |event: &str, target: &str, detail: &str, ok: bool| {
        crate::core::audit::log(&exec.data_dir, event, &exec.session_id, target, detail, ok);
    };
    let env = match coworker::Envelope::parse(args) {
        Ok(e) => e,
        Err(refusal) => return refusal,
    };
    let hops = coworker::depth_of(&exec.session_id);
    if let Some(refusal) = coworker::cap_refusal(hops, exec.asked_count()) {
        note("handoff.refused", args["coworker"].as_str().unwrap_or(""), &refusal, false);
        return refusal;
    }
    let reg = crate::core::personas::PersonaRegistry::load(&state.data_dir);
    let to = match coworker::resolve(&reg, &exec.persona, args["coworker"].as_str().unwrap_or("")) {
        Ok(c) => c,
        Err(refusal) => {
            note("handoff.refused", args["coworker"].as_str().unwrap_or(""), &refusal, false);
            return refusal;
        }
    };
    // The same ask twice in one turn is one hop, not two. The first answer is already in
    // this run's context, and "handed over" a second time would promise a second answer.
    if !exec.note_ask(&env.key(&to.id)) {
        return coworker::duplicate_refusal(&to.name);
    }
    let from_name = reg.manifest(&exec.persona).map(|m| m.name.clone()).unwrap_or_else(|| exec.persona.clone());

    // Its own conversation, written down before the run so the row exists while the work
    // is happening — the same reason a watch's session is saved before it starts.
    let sid = coworker::session_id(hops);
    let store = crate::core::sessions::SessionStore::new(&state.data_dir);
    let title = format!("{from_name} asked: {}", env.task.chars().take(60).collect::<String>());
    let mut session = crate::core::sessions::Session::with_id(&sid, &to.id, "", "");
    session.title = title.clone();
    store.save(&session);
    // What the person sees there is the one line, not the brief: the rest is instructions
    // to a model, and reading those is not reading a conversation.
    store.append(&sid, crate::core::sessions::Message::new("user", env.shown(&from_name)));

    // What the asking conversation has been about, in its own words. Without it the asked
    // assistant answers the task with none of the context the person already gave.
    let history: Vec<HistMsg> = store
        .get(&exec.session_id)
        .map(|prior| {
            let all = prior.history();
            all.iter()
                .skip(all.len().saturating_sub(6))
                .map(|(role, content)| HistMsg { role: role.clone(), content: content.clone(), wire: None })
                .collect()
        })
        .unwrap_or_default();

    note(
        "handoff.asked",
        &to.id,
        &format!("{} was asked: {}", to.name, env.task.chars().take(200).collect::<String>()),
        true,
    );

    // Stop in the asking conversation stops the hop too: the person pressed it about this
    // work, and a hop that runs on after it would be work nobody is waiting for.
    let stop_here = session_cancel_flag(&exec.session_id);
    let stop_there = session_cancel_flag(&sid);
    let watcher = tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if stop_here.load(Ordering::SeqCst) {
                stop_there.store(true, Ordering::SeqCst);
                return;
            }
        }
    });

    let hop = Hop {
        from_persona: exec.persona.clone(),
        from_name: from_name.clone(),
        from_session: exec.session_id.clone(),
        mode_ceiling: exec.mode.clone(),
        tainted: exec.is_tainted(),
    };
    let began = std::time::Instant::now();
    let run = Box::pin(run_agent_inner(
        app,
        state,
        to.id.clone(),
        env.brief(&from_name),
        Some(sid.clone()),
        // The model this run is on, not the app's default: a handoff is the same turn's
        // work carried on by somebody else.
        (!exec.model.is_empty()).then(|| exec.model.clone()),
        Some(history),
        0,
        None,
        None,
        Some(hop),
    ));
    let out = tokio::time::timeout(std::time::Duration::from_secs(coworker::DEADLINE_SECS), run).await;
    watcher.abort();
    let seconds = began.elapsed().as_secs_f64();

    match out {
        Ok(Ok(answer)) => {
            // An answer from a remote agent is content from off this machine, like a fetched
            // page: whatever this run does next with it is decided by a person.
            if crate::core::aguiclient::is_remote(&answer.provider) {
                exec.taint();
            }
            let mut done = crate::core::sessions::Message::new("assistant", answer.answer.clone()).with_usage(answer.usage.clone());
            done.items = answer.charts.clone();
            store.append(&sid, done);
            note(
                "handoff.delivered",
                &to.id,
                &format!("{} answered in {seconds:.0}s ({} characters)", to.name, answer.answer.chars().count()),
                true,
            );
            // A link to the conversation the answer is in, so the person can carry on with
            // the assistant that gave it rather than through the one that asked.
            exec.note_item(serde_json::json!({ "kind": "thread", "session_id": sid, "title": title }));
            offloaded(&exec.root, &format!("{}-answer", to.id), coworker::relayed(&to.name, &answer.answer))
        }
        Ok(Err(e)) => {
            store.append(&sid, crate::core::sessions::Message::new("assistant", format!("That run failed: {e}")));
            note("handoff.failed", &to.id, &format!("{}: {e}", to.name), false);
            coworker::failed(&to.name, &e)
        }
        Err(_) => {
            store.append(
                &sid,
                crate::core::sessions::Message::new("assistant", "No answer — this ran past the five minutes a handoff gets and was stopped."),
            );
            note("handoff.failed", &to.id, &format!("{} ran past the five-minute deadline", to.name), false);
            coworker::timed_out(&to.name)
        }
    }
}

/// An MCP server describes its tools with a name, a description and a schema —
/// and no notion of risk. Classify by verb: names that only fetch are Read (and so
/// ungated), anything else could create, edit, send or delete inside someone's
/// account and is External, which puts it through the same approval gate as our
/// own consequential tools. Unknown verbs fail safe to External.
pub(in crate::commands) fn mcp_risk(raw: &str) -> crate::core::agent::RiskClass {
    use crate::core::agent::RiskClass;
    const READ_ONLY: [&str; 12] =
        ["search", "list", "get", "read", "fetch", "query", "find", "view", "describe", "show", "retrieve", "count"];
    // Compare on the bare verb: MCP names are conventionally `verb_noun` or
    // `verb-noun`, often already namespaced (`notion_search_pages`).
    let lower = raw.to_lowercase();
    if READ_ONLY.iter().any(|v| lower.split(['_', '-', '.']).any(|part| part == *v)) {
        RiskClass::Read
    } else {
        RiskClass::External
    }
}
