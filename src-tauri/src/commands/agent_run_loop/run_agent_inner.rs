//! One agent turn, start to finish. This file is the order things happen in; each step
//! is a module: `model` (which model really runs), `tools` (what it may call), `prompt` +
//! `workspace` (the system prompt), `tool_ctx` (what tools run with), `context` (the history
//! that fits), then one provider branch — `handover` (Codex / Claude Code), `openai_loop`
//! or `claude_loop` — and `finish` (the report). `turn` holds what the branches share.

use super::super::*;

mod emit;
mod model;
mod tools;
mod prompt;
mod workspace;
mod tool_ctx;
mod context;
mod turn;
mod handover;
mod openai_wire;
mod openai_send;
mod openai_loop;
mod claude_send;
mod claude_loop;
mod finish;

use emit::*;
use model::*;
use tools::*;
use prompt::*;
use workspace::*;
use tool_ctx::*;
use context::*;
use turn::*;
use openai_wire::*;
use openai_send::*;
use openai_loop::*;
use claude_send::*;
use claude_loop::*;
use finish::*;

/// `depth` is the subagent nesting level (0 = the user's own run).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_inner(
    app: &tauri::AppHandle,
    state: &AppState,
    persona: String,
    message: String,
    session_id: Option<String>,
    model: Option<String>,
    history: Option<Vec<HistMsg>>,
    depth: u8,
    sink: Option<EventSink>,
    // Set when this run *is* a subagent: which type it runs as. It then works under
    // that type's prompt and toolset instead of the persona's.
    subagent_type: Option<String>,
    // Set when this run was asked for by another assistant — see `Hop`.
    hop: Option<Hop>,
) -> Result<AgentRunOut, String> {
    use std::sync::atomic::Ordering;
    use crate::core::providers::ProviderKind;
    let emit = |kind: &str, text: String| turn_event(app, &sink, kind, text);

    // An unknown persona id resolves to the default rather than failing the turn —
    // sessions can carry ids from another build (or the GUI's own default), and a
    // live session must keep working.
    let registry = crate::core::personas::PersonaRegistry::load(&state.data_dir);
    let manifest = registry
        .manifest(&persona)
        .or_else(|| registry.manifest(crate::core::personas::DEFAULT_PERSONA_ID))
        .cloned()
        .ok_or_else(|| format!("no persona available to run '{persona}'"))?;
    let agent = crate::core::agent::Agent::from_manifest(&manifest);

    let settings = state.settings.lock().unwrap().clone();
    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| crate::core::providers::effective_model(&settings));
    crate::trace!("turn", "resolved model={model} effort={} persona={}", settings.effort, manifest.id);

    // A private working folder for this run — deliverables land here.
    let sid = session_id.unwrap_or_else(|| format!("run-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)));

    // Somebody is waiting on this one, so the local model is spoken for: the
    // background sweeps hold off until the turn is done and a cooldown has passed.
    // See `core::modelgate` for why the cooldown.
    //
    // Claimed HERE, at the top, rather than around the model call. Two reasons, and
    // the first is easy to get wrong: the capability probe below is itself a model
    // call, so a claim taken any later leaves the probe — often the request that
    // *loads* the model — racing the very sweeps this is meant to hold off. The
    // second is that a turn is several model calls with tool work between them, and
    // releasing the model in those gaps is how a fifteen-minute sweep lands in the
    // middle of a conversation.
    //
    // An unattended run takes no claim: it IS the background work being kept out of
    // the user's way. Released by `Drop` at every exit from this function, including
    // the `?` returns between here and the loop.
    // A hop is unattended whoever is watching the turn that asked for it: the person is
    // in the asking conversation, not in this one, and cannot be put a question here.
    let unattended_run = sid.starts_with("__run__") || sid.starts_with("__task__") || hop.is_some();
    let _foreground = (!unattended_run).then(crate::core::modelgate::foreground);

    let own_folder = automation_root(state, &sid);
    let working_dir = own_folder.clone().unwrap_or_else(|| crate::core::agent_exec::session_dir(&state.data_dir, &sid));
    // A private folder's whole contents are the turn's output; a folder the user
    // already had is not. Only the second case needs a "since".
    let run_started = own_folder.is_some().then(std::time::SystemTime::now);
    // This run's own stop flag, cleared at the start of each turn so a Stop from a
    // previous turn can't kill this one before it begins.
    let cancel = session_cancel_flag(&sid);
    cancel.store(false, std::sync::atomic::Ordering::SeqCst);

    // An assistant whose turns are done by a remote AG-UI agent. None
    // of what follows applies to it — no model here, no tools, no gateway — because none of
    // it happens here, and pretending otherwise would be the lie. A subagent never runs
    // remotely: it is this turn's work broken up, and its type decides how it runs.
    if subagent_type.is_none() {
        if let Some(endpoint) = registry.remote(&manifest.id) {
            return run_remote(app, state, &manifest, &endpoint, &message, &sid, history.as_deref(), &working_dir, &sink, &cancel).await;
        }
    }

    let ModelChoice { underpowered, http, prov, api_model, profile, model_can_think } =
        choose_model(app, &sink, &manifest, &settings, &model).await?;
    let Toolset { shape, mut specs, held, skills, skills_text, mcp, sub } =
        choose_tools(state, &agent, &persona, &settings, &profile, &http, &working_dir, depth, &subagent_type, &message).await;
    let (mut system, mut volatile) = identity_prompt(
        state, &settings, &sub, &manifest, &sid, depth, &history, &message, &mut specs, &skills_text, &mcp, &persona, underpowered,
    );
    workspace_prompt(&mut system, &mut volatile, &specs, &sub, &working_dir);
    let exec = tool_ctx(state, &settings, &working_dir, &sid, &persona, &manifest, &hop, &sink, &message, &model, unattended_run, &mcp);

    state.cu_cancel.store(false, Ordering::SeqCst);
    emit("start", format!("{} is working…", manifest.name));

    let Fitted { budget, history_msgs, speculator, compaction_watch, max_steps } = fit_context(
        app, &sink, state, &settings, &profile, &shape, &sub, &history, &system, &message, &mut volatile, depth, &sid,
    )
    .await;

    // Stable first, volatile last — the shape a cache breakpoint can sit in.
    let sys_prompt = crate::core::llm::SystemPrompt { stable: system, volatile };
    crate::trace!(
        "turn",
        "system prompt stable={} chars volatile={} chars",
        sys_prompt.stable.len(),
        sys_prompt.volatile.len()
    );

    // What the turn actually took — see `Tally`.
    let turn_began = std::time::Instant::now();
    let mut t = Turn {
        app, state, sink, exec, persona, message, sid, model, api_model, depth, settings, prov, profile, model_can_think,
        shape, http, specs, held, skills, mcp, cancel, working_dir, budget, max_steps, sys_prompt, speculator,
        compaction_watch, tally: Tally::default(),
    };
    let ran = match prov.map(|p| p.kind) {
        Some(ProviderKind::CodexCli) => Ran::answer(handover::codex(&t, history_msgs).await?),
        Some(ProviderKind::ClaudeCli) => Ran::answer(handover::claude(&t, history_msgs).await?),
        Some(ProviderKind::Anthropic) => claude_loop(&mut t, history_msgs).await?,
        // Ollama and every OpenAI-style cloud provider.
        _ => openai_loop(&mut t, history_msgs).await?,
    };
    Ok(finish(t, ran, turn_began, run_started))
}
