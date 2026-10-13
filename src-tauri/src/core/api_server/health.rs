use super::*;

/// The GUI reads `model` from here to seed the composer, and `default_workspace`
/// to decide whether it must ask for a folder — both must be present or it renders
/// nothing.
/// What an unauthenticated caller is told: that the server is up, and nothing
/// else. Separate from the handler so the boundary is testable on its own.
pub(super) fn public_health() -> Value {
    json!({ "ok": true, "service": "superuser", "status": "ok" })
}

pub(super) async fn health(AxState(ctx): AxState<Arc<ApiContext>>, headers: HeaderMap) -> Json<Value> {
    use tauri::Manager;
    // This is the one route the token guard exempts, so that a client can tell
    // "not running" from "running, wrong token". That exemption is only safe for
    // a *liveness* answer. CORS here is `AllowOrigin::any()` on a fixed port, so
    // any page the user happens to visit can read whatever this returns — and it
    // used to return `default_workspace`, an absolute path carrying the account
    // name and what the user is working on. Liveness is public; the rest isn't.
    if !authed(&headers, &ctx.token) {
        return Json(public_health());
    }

    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    // The third copy of this decision, and the one that was still wrong. The GUI
    // takes the model it DISPLAYS from here, so while `settings` and the turn
    // itself had been corrected to ask whether the saved cloud model's own
    // provider is ready, this still asked whether an Anthropic key exists — and
    // answered `ollama_model` for everyone without one. The app showed
    // "gemma4:12b" in the header and the composer while running a completely
    // different model, which is the worst version of this bug: not a broken
    // feature, a confidently wrong label.
    let model = crate::core::providers::effective_model(&settings);
    let default_workspace = crate::core::projects::list(&state.data_dir).first().and_then(|p| p.folder.clone());
    Json(json!({
        "ok": true,
        "service": "superuser",
        "status": "ok",
        "model": model,
        "default_workspace": default_workspace,
        // Background work that crashed and was started again since launch. Empty is the
        // healthy answer; anything here is a crash that would otherwise be invisible.
        "restarts": crate::core::supervise::restarts(),
        // How saved credentials are kept: "encrypted", "empty", "plain", or "locked: why".
        // Locked is the one a person must hear about — every key reads as missing until then.
        "secrets": crate::core::secrets::sealing().label(),
    }))
}

/// Epoch seconds → ISO-8601, the format every GUI timestamp field expects.
pub(super) fn iso_secs(ts: f64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

/// Sessions with a turn in flight right now.
///
/// A turn outlives the socket that started it, so "is this working?" can't be
/// answered by whether a socket is open. The sidebar reads this to show its
/// working dot, and a reconnecting client uses it to know the session is still
/// busy rather than idle-and-empty.
/// Maps a running session to WHEN its turn started (epoch millis).
///
/// It was a bare set, and the missing half showed up as a stopwatch that lied.
/// Reattaching to a running turn replays `turn_start`, the GUI starts its elapsed
/// clock from the moment it receives one, and nothing carried the real start — so
/// every switch between two working chats reset both timers to zero. With two turns
/// in flight the number on screen was never the age of either.
pub(super) fn running_set() -> &'static std::sync::Mutex<std::collections::HashMap<String, u64>> {
    static RUNNING: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u64>>> =
        std::sync::OnceLock::new();
    RUNNING.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// When the in-flight turn for this session began, if one is in flight.
pub(crate) fn running_since(id: &str) -> Option<u64> {
    running_set().lock().ok().and_then(|s| s.get(id).copied())
}

/// Live frames for an in-flight turn, so reattaching replays what you missed.
///
/// A turn keeps running when you switch chats, but its events used to be written
/// straight to the socket — with nobody attached they went nowhere, so coming back
/// showed an empty transcript and an empty Progress panel even though the work was
/// still happening. Each running turn now buffers its frames and broadcasts them;
/// a (re)connecting socket replays the buffer, then follows live.
/// Buffer, live channel, and the folder whose journal outlives both — see
/// [`crate::core::runlog`]. The buffer answers "what did I miss" for a turn still
/// running; the journal answers it for one that finished yesterday.
pub(super) type RunFeed = (
    std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
    tokio::sync::broadcast::Sender<Value>,
    std::path::PathBuf,
);

pub(super) fn feeds() -> &'static std::sync::Mutex<std::collections::HashMap<String, RunFeed>> {
    static FEEDS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, RunFeed>>> =
        std::sync::OnceLock::new();
    FEEDS.get_or_init(Default::default)
}

/// Start (or restart) a session's feed, clearing anything from the previous turn.
pub(super) fn feed_open(id: &str, run_dir: std::path::PathBuf) -> RunFeed {
    let (tx, _) = tokio::sync::broadcast::channel(512);
    crate::core::runlog::start(&run_dir);
    let feed: RunFeed = (std::sync::Arc::new(std::sync::Mutex::new(Vec::new())), tx, run_dir);
    feeds().lock().unwrap().insert(id.to_string(), feed.clone());
    feed
}

pub(super) fn feed_get(id: &str) -> Option<RunFeed> {
    feeds().lock().unwrap().get(id).cloned()
}

pub(super) fn feed_close(id: &str) {
    feeds().lock().unwrap().remove(id);
}

/// Append a frame to the buffer, write it to the run's journal, and push it to
/// whoever is listening. One choke point, so nothing a run emits can be missing
/// from the record by having taken a different path.
pub(super) fn feed_push(feed: &RunFeed, frame: Value) {
    if let Ok(mut buf) = feed.0.lock() {
        // Cap the replay so a very long turn can't grow without bound.
        if buf.len() < 2000 {
            buf.push(frame.clone());
        }
    }
    crate::core::runlog::append(&feed.2, &frame);
    let _ = feed.1.send(frame);
}

/// Things typed while a turn was already in flight.
///
/// Until now the only thing you could do to a working agent was stop it: a message
/// arriving mid-turn was dropped on the floor. But the moment you most want to say
/// something is usually the moment you can see it going the wrong way, and making
/// someone wait for a wrong answer to finish before they can correct it is a poor
/// trade.
///
/// A steer is injected as a new user instruction at the next **iteration boundary**
/// — after the current step's tool results have been committed to the history, and
/// before the model is called again. That point matters: both provider APIs require
/// every `tool_use` to be answered by its `tool_result`, so slipping a message in
/// mid-step would corrupt the exchange.
pub(super) fn steer_queue() -> &'static std::sync::Mutex<std::collections::HashMap<String, Vec<String>>> {
    static Q: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Vec<String>>>> =
        std::sync::OnceLock::new();
    Q.get_or_init(Default::default)
}

/// Hold a message for the running turn to pick up.
pub(crate) fn queue_steer(id: &str, text: &str) {
    if let Ok(mut q) = steer_queue().lock() {
        let slot = q.entry(id.to_string()).or_default();
        // A bound, so holding down enter can't queue an unbounded pile of
        // instructions for one turn to swallow.
        if slot.len() < 8 {
            slot.push(text.to_string());
        }
    }
}

/// Take everything queued for this session, leaving the queue empty.
pub fn take_steers(id: &str) -> Vec<String> {
    steer_queue().lock().ok().and_then(|mut q| q.remove(id)).unwrap_or_default()
}

/// Drop anything queued for a session — called when a turn ends, so a steer that
/// arrived microseconds too late can't leak into the *next*, unrelated turn.
pub(crate) fn clear_steers(id: &str) {
    if let Ok(mut q) = steer_queue().lock() {
        q.remove(id);
    }
}

pub(super) fn mark_running(id: &str, running: bool) {
    if let Ok(mut set) = running_set().lock() {
        if running {
            // Keep the FIRST start: a turn that spans several model calls is one turn,
            // and the elapsed time the user is watching is the age of the whole thing.
            set.entry(id.to_string()).or_insert_with(now_ms);
        } else {
            set.remove(id);
        }
    }
}
