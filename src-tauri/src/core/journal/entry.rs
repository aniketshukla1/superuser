use super::*;

/// Entries the core file is distilled toward. Past this it stops being a summary
/// and starts being a second transcript — and it is paid for on every single turn.
pub const CORE_TARGET_ENTRIES: usize = 30;

/// Hard ceiling on what gets injected into a prompt, in characters. A core memory
/// that grew pathologically (a distillation that echoed its input, say) must not be
/// able to eat a small model's whole context window.
pub(super) const CORE_INJECT_CHAR_CAP: usize = 8_000;

/// Ceiling on one daily entry. Evidence is worth keeping, but not at the cost of
/// duplicating whole sessions into the journal — past this the entry keeps its head
/// and tail, and the session named in the heading remains the full record.
pub(super) const EVIDENCE_CHAR_CAP: usize = 8_000;

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// One dated file in the journal.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub date: String,
    pub body: String,
}

pub struct Journal {
    pub root: PathBuf,
}
