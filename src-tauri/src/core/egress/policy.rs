//! Policy

use super::*;

/// Local-only mode, readable from anywhere.
///
/// A process-wide switch rather than a parameter because the code that needs to
/// honour it doesn't have `Settings` in scope and shouldn't have to: `sync_one`
/// takes an id and a memory engine, and threading settings through every
/// integration to reach the ones that dial out is how a switch ends up enforced in
/// four places out of twenty. It is kept in step by [`Settings::save`] and
/// [`Settings::load`], the two functions every change passes through, so it cannot
/// drift from what the user set.
pub(super) fn local_only_flag() -> &'static AtomicBool {
    static LOCAL_ONLY: OnceLock<AtomicBool> = OnceLock::new();
    LOCAL_ONLY.get_or_init(|| AtomicBool::new(false))
}

/// Point the spine at the app's data dir, so a refusal or a send can be recorded.
///
/// Until this is called nothing is written — the module still *enforces*, it just
/// has nowhere to file the record. Called once as `AppState` is built.
pub(super) fn log_dir() -> &'static std::sync::Mutex<Option<PathBuf>> {
    static DIR: OnceLock<std::sync::Mutex<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| std::sync::Mutex::new(None))
}

pub fn set_data_dir(dir: &Path) {
    if let Ok(mut d) = log_dir().lock() {
        *d = Some(dir.to_path_buf());
    }
}

pub fn set_local_only(on: bool) {
    local_only_flag().store(on, Ordering::SeqCst);
}

pub fn local_only() -> bool {
    local_only_flag().load(Ordering::SeqCst)
}
