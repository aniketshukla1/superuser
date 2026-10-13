use super::*;

/// How many drops to keep. Enough to show a pattern ("this one has tried nine
/// times"), small enough that nobody can fill a disk with it.
pub(super) const KEEP: usize = 50;

/// How much of a message to keep. The user needs to recognise it, not read it in
/// full — and the shorter it is, the less there is to regret storing.
pub(super) const TEXT_CHARS: usize = 280;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// One dropped message, in the shape the Inbox reads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnroutedItem {
    /// Stable handle for acting on this one item — allow the sender, deliver it,
    /// dismiss it. Derived from the drop itself rather than a counter, because the
    /// file is rewritten whole on every record and an index would shift under a
    /// card the user is looking at.
    #[serde(default)]
    pub id: String,
    /// Which connector it arrived on: "telegram", "slack".
    pub source: String,
    /// Who sent it, as that platform identifies them — a display name, for reading.
    pub sender: String,
    /// The id that platform knows them by, which is the thing an allowlist holds.
    ///
    /// Recorded separately from `sender` because the first version stored only
    /// "Ann (5551234)" — readable, and useless: the Inbox could show that someone
    /// had tried and offer no way to let them in, because the only actionable half
    /// was glued inside a sentence.
    #[serde(default)]
    pub chat_id: String,
    pub text: String,
    /// Why it went nowhere, in words the user can act on.
    pub reason: String,
    pub ts: f64,
}

/// A short stable id for one drop. Not a security boundary — it names a row in a
/// local file — so a cheap hash is the right tool.
pub(super) fn id_for(source: &str, chat_id: &str, text: &str, ts: f64) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    chat_id.hash(&mut h);
    text.hash(&mut h);
    ts.to_bits().hash(&mut h);
    format!("{:016x}", h.finish())
}

pub struct Unrouted {
    pub(super) path: PathBuf,
}

impl Unrouted {
    pub fn load(data_dir: &Path) -> Self {
        Self { path: data_dir.join("unrouted.json") }
    }

    /// Newest first — the Inbox shows the most recent attempt at the top, and a
    /// list of strangers is only worth reading from the top.
    pub fn list(&self) -> Vec<UnroutedItem> {
        let mut items: Vec<UnroutedItem> = fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        // Items recorded before ids existed still have to be actionable, so one is
        // derived on read. Deterministic, so the same row keeps the same handle.
        for it in items.iter_mut() {
            if it.id.is_empty() {
                it.id = id_for(&it.source, &it.chat_id, &it.text, it.ts);
            }
        }
        items.sort_by(|a, b| b.ts.total_cmp(&a.ts));
        items
    }

    /// The drops from one connector, newest first.
    pub fn for_source(&self, source: &str) -> Vec<UnroutedItem> {
        self.list().into_iter().filter(|i| i.source == source).collect()
    }

    /// Remove one item and hand it back — the only way anything leaves this store
    /// with its text intact, and it exists so that a user who clicked "allow and
    /// deliver" gets that message answered ONCE. See the module note.
    pub fn take(&self, id: &str) -> Option<UnroutedItem> {
        let mut items = self.list();
        let idx = items.iter().position(|i| i.id == id)?;
        let item = items.remove(idx);
        self.write(&items);
        Some(item)
    }

    /// Drop one item without reading it out. "I have seen this; go away."
    pub fn dismiss(&self, id: &str) -> bool {
        self.take(id).is_some()
    }

    pub(super) fn write(&self, items: &[UnroutedItem]) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(items) {
            let _ = crate::core::store::write_atomic(&self.path, &json);
        }
    }

    /// Record a drop. Never fails a caller: this is a courtesy to the owner, and a
    /// full disk or a read-only folder must not break message polling.
    pub fn record(&self, source: &str, chat_id: &str, sender: &str, text: &str, reason: &str) {
        let mut items = self.list();
        let ts = now();
        let text: String = text.chars().take(TEXT_CHARS).collect();
        items.insert(
            0,
            UnroutedItem {
                id: id_for(source, chat_id, &text, ts),
                source: source.to_string(),
                sender: sender.to_string(),
                chat_id: chat_id.to_string(),
                text,
                reason: reason.to_string(),
                ts,
            },
        );
        items.truncate(KEEP);
        self.write(&items);
    }

    /// Forget everything — the user has seen the list and dealt with it.
    pub fn clear(&self) {
        let _ = fs::remove_file(&self.path);
    }
}
