use super::super::*;

/// Did a tool call succeed? `run_tool` hands failures back to the model as plain text
/// rather than a Result, so this reads that convention ("error: …", "… failed: …").
///
/// Deliberately conservative: the GUI paints anything that isn't `ok` as a red failed
/// step, and wrongly flagging a good step is worse than missing a bad one.
pub(crate) fn tool_status(out: &str) -> &'static str {
    // One reading, shared with the gateway, so the transcript and the audit trail can
    // never disagree about whether a step failed — and so a boundary that held reads
    // as `refused` rather than as a tool that broke.
    crate::core::gateway::status_of(out)
}

pub(in crate::commands) fn tool_label(name: &str, args: &serde_json::Value) -> String {
    let s = |k: &str| args[k].as_str().unwrap_or("").to_string();
    match name {
        "read_file" => format!("read {}", s("path")),
        "write_file" => format!("write {}", s("path")),
        "edit_file" => format!("edit {}", s("path")),
        "list_dir" => format!("list {}", s("path")),
        "run_shell" => format!("$ {}", s("command").chars().take(80).collect::<String>()),
        // A delegated Codex turn runs several of these, and without the command
        // itself the live panel is a column of identical "codex_shell" rows that
        // say only that something happened.
        "codex_shell" => format!("codex $ {}", s("command").chars().take(80).collect::<String>()),
        "grep" => format!("grep \"{}\"", s("pattern")),
        "web_search" => format!("web: {}", s("query")),
        "web_fetch" => format!("fetch {}", s("url")),
        "browser_open" => format!("show {}", s("url")),
        "doc_read" => format!("read {}", s("path")),
        "doc_edit" => format!("edit {}", s("path")),
        "slides_read" => format!("read {}", s("path")),
        "slides_edit" => format!("edit {}", s("path")),
        "recall_memory" => format!("recall: {}", s("query")),
        "todo_write" => "update plan".into(),
        "propose_plan" => "propose a plan".into(),
        "load_skill" => format!("load skill {}", s("name")),
        other => other.to_string(),
    }
}

/// Wall-clock budget for one `spawn_subagent` call, covering all its tasks
/// together. A parent waiting on a subagent is a parent doing nothing, so a stuck
/// task must not hold a conversation open indefinitely.
pub(in crate::commands) const SUBAGENT_BUDGET: std::time::Duration = std::time::Duration::from_secs(300);

/// Most tasks one call may fan out to. Beyond this the local model thrashes and
/// everything finishes slower than doing them in order.
pub(in crate::commands) const MAX_PARALLEL_SUBAGENTS: usize = 3;

/// Pull `(task, type)` pairs out of a `spawn_subagent` call.
///
/// The documented shape is `{tasks: [{task, type}]}`, but models reach for the
/// simpler `{task: "…"}` and for a bare array of strings often enough that
/// rejecting those just burns a turn on a reformat. Anything blank is dropped.
pub(in crate::commands) fn parse_subagent_tasks(args: &serde_json::Value) -> Vec<(String, String)> {
    const DEFAULT_TYPE: &str = "general-purpose";
    let typed = |v: &serde_json::Value, fallback: &str| v.as_str().filter(|s| !s.trim().is_empty()).unwrap_or(fallback).to_string();

    if let Some(arr) = args["tasks"].as_array() {
        return arr
            .iter()
            .filter_map(|t| {
                let task = t["task"].as_str().or_else(|| t.as_str())?.trim().to_string();
                (!task.is_empty()).then(|| (task, typed(&t["type"], DEFAULT_TYPE)))
            })
            .collect();
    }
    args["task"]
        .as_str()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| vec![(t.to_string(), typed(&args["type"], DEFAULT_TYPE))])
        .unwrap_or_default()
}

/// Run a `spawn_subagent` call: every task in it at once, under one shared
/// deadline, each with its own type and toolset.
///
/// The result count always matches the task count, and a task that ran out of time
/// says so. Silently returning fewer results would let the parent read "found
/// nothing" into what was actually "never finished".
/// Carry out an ask the user has approved.
///
/// The gate parks a call and returns "parked" to the model straight away — there
/// is no suspended step to resume — so approval has to *perform* the action, not
/// unblock it. Until this existed only `send_message` was carried out: everything
/// else was recorded as approved and silently never happened, which is worse than
/// a refusal because it looks like it worked.
///
/// The call is replayed through `run_tool`, the same executor that parked it,
/// rather than a second switch statement that would drift from the first. The
/// context it runs under encodes exactly what the user just decided:
///
/// * `mode: "auto"` and a standing rule for this one tool+target — the gate has
///   already been answered, so re-asking would park it a second time;
/// * `tainted: false` — taint's whole purpose is to force a human decision about a
///   run that read outside content, and a human has now made it. Leaving it set
///   would re-park an action the user is looking at having approved;
/// * `unattended: false` — someone is plainly present.
///
/// Scoped to this one call: the ToolCtx is built here and dropped after, so
/// nothing about it outlives the action it authorises.
pub(crate) async fn perform_approved(
    app: &tauri::AppHandle,
    state: &AppState,
    ask: &crate::core::inbox::Ask,
) -> Result<String, String> {
    // An escalation from the subconscious isn't a tool call at all — approving it
    // means doing the work the watch noticed was needed.
    if ask.tool == "subconscious_escalation" {
        let question = ask.args["watch"].as_str().unwrap_or(&ask.summary);
        let evidence = ask.args["evidence"].as_str().unwrap_or("");
        let reason = ask.args["reason"].as_str().unwrap_or("");
        let message = format!(
            "A background watch flagged this and you approved looking into it.\n\n\
             ## The watch\n\n{question}\n\n## What changed\n\n{evidence}\n\n## Why it was flagged\n\n{reason}\n\n\
             Work out what's actually needed and do it. If acting would affect anything outside this machine, \
             say what you'd do and stop."
        );
        // Its own session — and a *visible* one. A run started outside the socket
        // leaves no session record, and the sidebar hides anything prefixed `__`,
        // so the obvious `__watch__…` id would have replaced "does nothing" with
        // "does something you can't see". The record is written before the run so
        // the row appears while the work is still happening.
        let sid = format!("watch-{}", crate::core::inbox::short_id());
        let persona = crate::core::personas::DEFAULT_PERSONA_ID.to_string();
        let store = crate::core::sessions::SessionStore::new(&state.data_dir);
        let mut session = crate::core::sessions::Session::with_id(&sid, &persona, "", "");
        session.title = format!("Watch: {}", question.chars().take(60).collect::<String>());
        store.save(&session);
        store.append(&sid, crate::core::sessions::Message::new("user", message.clone()));
        let out = run_agent(app, state, persona, message, Some(sid.clone()), None, None).await;
        if let Ok(o) = &out {
            store.append(&sid, crate::core::sessions::Message::new("assistant", o.answer.clone()));
        }
        return out.map(|o| o.answer);
    }

    let settings = state.settings.lock().unwrap().clone();
    let working_dir = crate::core::agent_exec::session_dir(&state.data_dir, &ask.session_id);
    let exec = crate::core::agent_exec::ToolCtx {
        root: working_dir,
        memory: &state.memory,
        settings,
        todo: std::sync::Mutex::new(Vec::new()),
        data_dir: state.data_dir.clone(),
        session_id: ask.session_id.clone(),
        persona: crate::core::personas::DEFAULT_PERSONA_ID.to_string(),
        task_id: ask.task_id.clone(),
        mode: "auto".into(),
        standing: vec![crate::core::automation::rule_entry(&ask.tool, Some(&ask.target))],
        unattended: false,
        tainted: std::sync::atomic::AtomicBool::new(false),
        gropes: std::sync::atomic::AtomicU32::new(0),
        // Replaying one approved call, with no live turn behind it: there is no
        // socket to ask a question down, and the user's words are the approval
        // itself. The gates still apply — they just have nothing quoted to compare
        // an argument against, which is the strict reading, not a lenient one.
        events: None,
        prompt: String::new(),
        seen: std::sync::Mutex::new(Vec::new()),
        calls: std::sync::Mutex::new(Default::default()),
        stalls: std::sync::Mutex::new(Default::default()),
        charts: std::sync::Mutex::new(Vec::new()),
        urls: std::sync::Mutex::new(Vec::new()),
        coverage: std::sync::Mutex::new(Vec::new()),
        reviewer: Default::default(),
        model: String::new(),
        // The user approved this in the Inbox: a person started it, whichever run parked it.
        initiator: Some(crate::core::audit::Initiator::new("approval", &ask.id)),
        asked: std::sync::Mutex::new(Vec::new()),
    };

    let skills = crate::core::skills::SkillStore::load(&state.data_dir);
    let http = crate::core::egress::client("mcp");
    // Connected apps need a live session to act on; a parked MCP call is the main
    // reason this path exists, so the cost of opening them is the point.
    let mut mcp = if crate::core::inbox::is_connector_tool(&ask.tool) {
        gather_mcp(&http, &state.data_dir).await
    } else {
        McpBundle::default()
    };
    Ok(run_tool(app, state, &exec, &skills, &mut mcp, &http, 0, &ask.tool, &ask.args).await)
}

/// Messages the user typed while this turn was working. A subagent's session has
/// no socket of its own, so only the top-level run can be steered — a steer aimed
/// at the parent must not be swallowed by a delegate.
pub(in crate::commands) fn steers(sid: &str) -> Vec<String> {
    crate::core::api_server::take_steers(sid)
}
