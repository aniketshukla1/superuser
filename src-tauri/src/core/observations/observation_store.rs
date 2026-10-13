use super::*;

/// How much truncated output one session may keep. Ten megabytes is roughly five
/// hundred maximum-size truncations; a session that goes past it is not doing
/// something a bigger cap would fix.
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Length of a handle, in hex characters. Six bytes of SHA-256. Short enough that
/// the notice stays readable at a glance, long enough that a session would need
/// billions of distinct observations before two collided.
pub(super) const ID_LEN: usize = 12;

/// Per-session store of tool output that was truncated out of the window.
#[derive(Debug, Clone)]
pub struct ObservationStore {
    pub(super) dir: PathBuf,
    pub(super) cap: u64,
}

/// Filesystem-safe form of a session id.
///
/// Same rule as [`SessionStore::dir`](crate::core::sessions::SessionStore) and for
/// the same reason: the mapping has to be injective. Replacing every unsafe
/// character with `_` would let `chat/one` and `chat:one` share a directory, and
/// two conversations sharing an observation store is a cross-conversation leak of
/// exactly the material most likely to be sensitive — whole command outputs.
pub(super) fn safe_id(id: &str) -> String {
    let ok = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if !id.is_empty() && id.chars().all(ok) {
        return id.to_string();
    }
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(id.as_bytes());
    let tag: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    let stem: String = id.chars().map(|c| if ok(c) { c } else { '_' }).take(48).collect();
    if stem.is_empty() {
        format!("s-{tag}")
    } else {
        format!("{stem}-{tag}")
    }
}

/// The handle for a piece of text. Content-addressed, so identical output taken
/// twice is one entry and one id.
pub fn handle_for(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(text.as_bytes());
    digest.iter().take(ID_LEN / 2).map(|b| format!("{b:02x}")).collect()
}

/// True if `id` is shaped like a handle this store could have minted.
///
/// Checked before the id ever reaches the filesystem: a handle arrives from the
/// model, and `../` in a tool argument must not become a path.
pub(crate) fn well_formed(id: &str) -> bool {
    id.len() == ID_LEN && id.chars().all(|c| c.is_ascii_hexdigit())
}

impl ObservationStore {
    /// The store for one session, under the data folder of the instance it runs in — the
    /// caller's, which is the one its tools get (`ToolCtx::data_dir`). Never worked out here:
    /// a folder resolved in this module ignored `SUPERUSER_DATA_DIR`, so a scratch instance
    /// wrote what it cut into the real app's folder, where neither `recall_output` nor a
    /// session delete ever looked.
    ///
    /// Cheap: this builds a path and touches nothing. The directory is created on the
    /// first `put`, so a session that never truncates anything never leaves a folder.
    pub fn for_session(data_dir: &Path, session_id: &str) -> Self {
        Self { dir: data_dir.join("observations").join(safe_id(session_id)), cap: MAX_BYTES }
    }

    /// Same store with a different ceiling. Tests use it so the cap can be
    /// exercised without allocating ten megabytes to prove one comparison.
    #[cfg(test)]
    pub(super) fn with_cap(mut self, cap: u64) -> Self {
        self.cap = cap;
        self
    }

    /// Where this store keeps its files. Exposed for tests and for `clear`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub(super) fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.txt"))
    }

    /// Current size of the store on disk.
    pub(super) fn used(&self) -> u64 {
        let Ok(rd) = fs::read_dir(&self.dir) else { return 0 };
        rd.filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }

    /// Record `text` and return its handle, or `None` if it could not be stored.
    ///
    /// `None` is not an error path the caller has to explain to anyone: it means
    /// the notice keeps the old "re-read the source" wording for this one result,
    /// which is what the whole product did until now.
    ///
    /// Storing the same text twice is free and yields the same handle — the second
    /// put sees the file already there and writes nothing.
    pub fn put(&self, text: &str) -> Option<String> {
        let id = handle_for(text);
        let p = self.path(&id);
        if p.exists() {
            return Some(id);
        }
        // Checked before the write, and against the incoming length, so the cap is
        // a ceiling rather than a line we cross once per session and then notice.
        if self.used().saturating_add(text.len() as u64) > self.cap {
            return None;
        }
        fs::create_dir_all(&self.dir).ok()?;
        fs::write(&p, text).ok()?;
        Some(id)
    }

    /// The whole text behind a handle.
    /// Whether a handle names something this store holds. A path check; nothing is read.
    pub fn has(&self, id: &str) -> bool {
        well_formed(id) && self.path(id).exists()
    }

    pub fn get(&self, id: &str) -> Option<String> {
        if !well_formed(id) {
            return None;
        }
        fs::read_to_string(self.path(id)).ok()
    }

    /// A character window of the text behind a handle, and the total length.
    ///
    /// Characters rather than bytes: the model counts what it can see, and a byte
    /// range would split a UTF-8 sequence on any output that is not ASCII.
    /// `from` past the end yields an empty string rather than `None` — that is a
    /// model asking for a range that does not exist, which is an answerable
    /// question, not a missing handle.
    pub fn slice(&self, id: &str, from: usize, len: usize) -> Option<(String, usize)> {
        let text = self.get(id)?;
        let total = text.chars().count();
        let out: String = text.chars().skip(from).take(len).collect();
        Some((out, total))
    }

    /// Drop everything this session recorded. Called when the session is deleted.
    pub fn clear(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
