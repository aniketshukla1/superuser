use super::*;

/// Shared application state, injected into every command via `State<AppState>`.
pub struct AppState {
    pub memory: MemoryEngine,
    pub settings: Mutex<Settings>,
    pub registry: Registry,
    pub data_dir: PathBuf,
    #[allow(dead_code)]
    pub mcp: mcp::McpHost,
    /// Cooperative cancel flag for the computer-use agent loop (set by cu_stop).
    pub cu_cancel: Arc<AtomicBool>,
    // The interactive terminal's PTY session is a process-global in core::terminal,
    // not a field here: the agent's `run_shell` shares that shell, and it is dispatched
    // from a plain sync function with no access to this state.
    /// The local OpenAI-compatible API, when the user has started it (off by default).
    pub api: Arc<Mutex<Option<core::api_server::RunningApi>>>,
    /// Microphone capture and the local speech model.
    pub dictation: core::dictation::Dictation,
    /// The voice that is speaking right now, so it can be stopped mid-word.
    pub speaker: core::tts::Speaker,
}

/// Where the app kept its data under its old name, BigBrainAI.
pub(super) const LEGACY_DATA_DIR: &str = "ai.bigbrain.app";

pub(super) const DATA_DIR: &str = "ai.superuser.app";

/// Where everything this app knows is kept, overridable for a sweep.
///
/// `SUPERUSER_DATA_DIR` moves the whole install — vault, journal, knowledge,
/// profile, sessions, settings — somewhere else for the life of the process. It
/// exists for the same reason `HARNESS_EVAL_PORT` does: a long scoreboard run
/// wants a headless instance that is not the user's app, and the brain tasks
/// (`BRAIN-RESEARCH-2026` §3) need a *seeded* brain, which is not something a task
/// can put in a working folder. Pointing a second instance at a scratch directory
/// is the only honest way to grade recall: the alternative is writing fixtures
/// into the vault someone actually uses.
///
/// No migration is attempted when it is set. A scratch directory that gets handed
/// a copy of the user's real install is not scratch.
pub(super) fn resolve_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SUPERUSER_DATA_DIR").map(PathBuf::from).filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    // ~/Library/Application Support/ai.superuser.app on macOS, etc.
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join(DATA_DIR);
    // "Is there an install here?" — not "does the directory exist?". Several code
    // paths create this directory eagerly just to write into it, so an empty one
    // is not evidence of anything. Treating it as an existing install would skip
    // the migration and silently present a fresh, empty app to someone whose data
    // is sitting untouched under the old name.
    if !dir.join("settings.json").exists() && !dir.join("vault").exists() {
        migrate_legacy_data(&base.join(LEGACY_DATA_DIR), &dir);
    }
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Carry an existing install across the rename.
///
/// Copies rather than moves, deliberately: the old directory stays exactly as it
/// was, so a bug here costs nothing — everything is still where it always was, and
/// the user can point the app back at it. It only ever runs when the new location
/// holds no install, so it can never overwrite live data.
pub(super) fn migrate_legacy_data(old: &std::path::Path, new: &std::path::Path) {
    if !old.is_dir() || !old.join("settings.json").exists() {
        return;
    }
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let name = entry.file_name();
            // The sidecar token belongs to whichever build wrote it and is minted
            // fresh each launch. Carrying it over just leaves a stale credential on
            // disk that authenticates nothing.
            if name.to_string_lossy().starts_with("sidecar-") {
                continue;
            }
            let (src, dst) = (entry.path(), to.join(&name));
            if entry.file_type()?.is_dir() {
                copy_tree(&src, &dst)?;
            } else {
                std::fs::copy(&src, &dst)?;
            }
        }
        Ok(())
    }
    match copy_tree(old, new) {
        Ok(()) => eprintln!("[superuser] carried your data over from {}", old.display()),
        Err(e) => {
            // Leave no half-copied directory behind: a partial copy would satisfy
            // the "is there an install here?" check on the next launch and strand
            // the rest of their data under the old name forever.
            let _ = std::fs::remove_dir_all(new);
            eprintln!("[superuser] couldn't migrate data from {}: {e}", old.display());
        }
    }
}
