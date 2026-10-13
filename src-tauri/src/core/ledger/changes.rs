use super::*;

/// Git's empty tree. Diffing against it means "everything is new", which is
/// exactly right for a source nobody has read yet.
pub(super) const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// What changed for one source since it was last read.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Changes {
    pub source: String,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    /// Files this source has in the vault right now, changed or not.
    pub total: usize,
    /// True when the marker moved — i.e. this call consumed the delta.
    pub acknowledged: bool,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    /// A line the agent (or a brief) can read. Deliberately terse: this ends up in
    /// prompts and in notifications.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return format!("{}: nothing new.", self.source);
        }
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!("{} new", self.added.len()));
        }
        if !self.modified.is_empty() {
            parts.push(format!("{} changed", self.modified.len()));
        }
        if !self.removed.is_empty() {
            parts.push(format!("{} removed", self.removed.len()));
        }
        format!("{}: {}.", self.source, parts.join(", "))
    }
}

pub struct Ledger {
    /// The vault directory — the repository root.
    pub root: PathBuf,
}
