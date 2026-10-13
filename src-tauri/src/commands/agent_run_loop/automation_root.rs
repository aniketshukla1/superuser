use super::super::*;

/// Run an assistant: resolve the persona, expand its tools for a fresh working
/// folder, then drive the model↔tool loop (native tool-calling, Ollama or Claude),
/// dispatching each call for real. Returns the answer plus the todos and the
/// artifacts (files) it produced. Streams `agent-run` events; cancellable via cu_stop.
#[tauri::command]
pub async fn agent_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    persona: String,
    message: String,
    session_id: Option<String>,
    model: Option<String>,
    history: Option<Vec<HistMsg>>,
) -> Result<AgentRunOut, String> {
    run_agent(&app, &state, persona, message, session_id, model, history).await
}

/// The assistant run loop — callable from the `agent_run` command or the background
/// automation scheduler (both hand it `&AppHandle` + `&AppState`).
/// One assistant asking another, carried into the delegated run.
///
/// Everything here NARROWS the run it starts. A hop that could act more freely than the
/// turn that asked for it would make delegation the way around every approval in the app:
/// a discuss-only assistant would only have to know somebody who says yes.
#[derive(Debug, Clone)]
pub(crate) struct Hop {
    /// Who asked, for the brief the asked assistant reads and for the trail.
    pub from_persona: String,
    pub from_name: String,
    /// The conversation the ask came from — what the hop's rows point back at.
    pub from_session: String,
    /// The asking run's permission mode. The hop runs under the stricter of this and its
    /// own assistant's.
    pub mode_ceiling: String,
    /// The asking run had read content from outside this machine, so the task text it
    /// wrote may be carrying it. Taint is sticky across a hop for the same reason it is
    /// sticky within a run.
    pub tainted: bool,
}

pub(crate) async fn run_agent(
    app: &tauri::AppHandle,
    state: &AppState,
    persona: String,
    message: String,
    session_id: Option<String>,
    model: Option<String>,
    history: Option<Vec<HistMsg>>,
) -> Result<AgentRunOut, String> {
    run_agent_inner(app, state, persona, message, session_id, model, history, 0, None, None, None).await
}

/// Progress events from a run, in the shape the GUI's session socket reads.
pub(crate) type EventSink = tokio::sync::mpsc::UnboundedSender<serde_json::Value>;

/// Like `run_agent`, but also streams each step to `sink` — used by the session
/// WebSocket so the transcript fills in live instead of only at the end.
/// Per-session cooperative cancel flags.
///
/// `cu_cancel` is a single global flag — fine when only one turn could exist, but
/// wrong now that a turn keeps running after you switch chats: stopping one session
/// would halt every other running session too. Each session gets its own flag; the
/// global stays for computer-use, which really is one-at-a-time.
pub(in crate::commands) fn cancel_flags() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>> {
    static FLAGS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>>> =
        std::sync::OnceLock::new();
    FLAGS.get_or_init(Default::default)
}

pub fn session_cancel_flag(session_id: &str) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let mut m = cancel_flags().lock().unwrap();
    m.entry(session_id.to_string())
        .or_insert_with(|| std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)))
        .clone()
}

/// Ask one session's turn to stop. Other sessions keep working.
pub fn cancel_session(session_id: &str) {
    session_cancel_flag(session_id).store(true, std::sync::atomic::Ordering::SeqCst);
}

pub(crate) async fn run_agent_streaming(
    app: &tauri::AppHandle,
    state: &AppState,
    persona: String,
    message: String,
    session_id: Option<String>,
    model: Option<String>,
    history: Option<Vec<HistMsg>>,
    sink: EventSink,
) -> Result<AgentRunOut, String> {
    run_agent_inner(app, state, persona, message, session_id, model, history, 0, Some(sink), None, None).await
}

/// Where an automation run should actually work.
///
/// `ScheduledTask.workspace` has been a field nobody read. It is written at
/// creation as `state.data_dir` — the app's own storage — and the run loop always
/// used a private per-run folder regardless, so "the morning brief writes to
/// brief.md" meant a brief.md nobody would ever find, in
/// `…/sessions/__run__run-6f9cb51d98/`. Pointing the task somewhere else changed
/// a label.
///
/// So: an automation run works in the folder its task names, when that is a real
/// folder. Scoped to `__run__` sessions on purpose — an ordinary chat keeps its
/// private sandbox, and widening that is a separate decision about what a
/// conversation is allowed to touch, not a side effect of fixing a label.
///
/// The data directory is refused as a workspace even when a task names it: that is
/// the creation default rather than a choice, and letting a scheduled agent write
/// into the app's own storage is not something anyone asked for.
pub(in crate::commands) fn automation_root(state: &AppState, sid: &str) -> Option<std::path::PathBuf> {
    if !sid.starts_with("__run__") {
        return None;
    }
    let ws = crate::core::sessions::SessionStore::new(&state.data_dir).get(sid)?.workspace;
    let path = std::path::PathBuf::from(ws.trim());
    if !path.is_dir() || path == state.data_dir {
        return None;
    }
    Some(path)
}
