use super::*;

/// What every tool call in this turn runs with, tainted up front by anything outside
/// the run has already said to the model.
#[allow(clippy::too_many_arguments)]
pub(super) fn tool_ctx<'a>(
    state: &'a AppState,
    settings: &crate::core::settings::Settings,
    working_dir: &std::path::PathBuf,
    sid: &String,
    persona: &String,
    manifest: &crate::core::personas::PersonaManifest,
    hop: &Option<Hop>,
    sink: &Option<EventSink>,
    message: &String,
    model: &String,
    unattended_run: bool,
    mcp: &McpBundle,
) -> crate::core::agent_exec::ToolCtx<'a> {
    // An automation-run session (`__run__…`) is unattended: nobody is watching, so
    // anything with consequences waits in the inbox rather than asking for a click.
    let unattended = unattended_run;
    let task = if unattended {
        crate::core::automation::AutomationStore::load(&state.data_dir)
            .list()
            .into_iter()
            .find(|t| sid.contains(&t.id) || t.task_session_id() == *sid)
    } else {
        None
    };
    let exec = crate::core::agent_exec::ToolCtx {
        root: working_dir.clone(),
        memory: &state.memory,
        settings: settings.clone(),
        todo: std::sync::Mutex::new(Vec::new()),
        data_dir: state.data_dir.clone(),
        session_id: sid.clone(),
        persona: persona.clone(),
        task_id: task.as_ref().map(|t| t.id.clone()),
        mode: match &hop {
            Some(h) => crate::core::coworker::stricter(&manifest.default_permission_mode, &h.mode_ceiling),
            None => manifest.default_permission_mode.clone(),
        },
        standing: task.as_ref().map(|t| t.always_allowed_tools.clone()).unwrap_or_default(),
        unattended,
        // A fresh run starts clean; reading anything from outside sets this.
        tainted: std::sync::atomic::AtomicBool::new(false),
        gropes: std::sync::atomic::AtomicU32::new(0),
        // `ask_user` sends its question down this; without it there is no one to ask.
        events: sink.clone(),
        prompt: message.clone(),
        seen: std::sync::Mutex::new(Vec::new()),
        calls: std::sync::Mutex::new(Default::default()),
        stalls: std::sync::Mutex::new(Default::default()),
        charts: std::sync::Mutex::new(Vec::new()),
        urls: std::sync::Mutex::new(Vec::new()),
        coverage: std::sync::Mutex::new(Vec::new()),
        reviewer: Default::default(),
        model: model.clone(),
        // A hop's rows say a handoff started them and which conversation asked, so the
        // trail reads back to the person's request rather than stopping at an assistant.
        initiator: hop.as_ref().map(|h| crate::core::audit::Initiator::new("handoff", &h.from_session)),
        asked: std::sync::Mutex::new(Vec::new()),
    };
    // Whatever the asking run had read from outside came with the task it wrote.
    if hop.as_ref().is_some_and(|h| h.tainted) {
        exec.taint();
    }

    // …unless a connected app's tool *descriptions* are already talking to the
    // model. Those go into the schema of every turn, so the run has effectively
    // read outside content before it takes a single step.
    if !mcp.surface_changed.is_empty() {
        // Same treatment as a description that reads as an instruction, for the same
        // reason: the run has been handed outside content it did not expect, before
        // it took a step.
        exec.taint();
        crate::core::audit::log(
            &state.data_dir,
            "external",
            &sid,
            "mcp",
            &format!(
                "tool surface changed since it was approved: {} — every call from it needs approval this run",
                mcp.surface_changed.join(", ")
            ),
            false,
        );
    }
    if !mcp.suspect_tools.is_empty() {
        exec.taint();
        crate::core::audit::log(
            &state.data_dir,
            "external",
            &sid,
            "mcp",
            &format!("tool descriptions read as instructions: {}", mcp.suspect_tools.join(", ")),
            false,
        );
    }
    exec
}
