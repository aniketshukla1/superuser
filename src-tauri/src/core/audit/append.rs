use super::*;

impl Entry {
    /// A typed event. `kind` is derived from it, so a reader that only knows the
    /// first version still groups the row somewhere sensible.
    pub fn event(event: &str) -> Self {
        Entry {
            kind: event.split('.').next().unwrap_or(event).to_string(),
            event: event.to_string(),
            ok: true,
            ..Default::default()
        }
    }

    /// The event name, for rows old and new alike.
    pub fn event_name(&self) -> &str {
        if self.event.is_empty() { &self.kind } else { &self.event }
    }

    pub fn refused(&self) -> bool {
        self.event_name() == "action.refused" || self.decision.as_ref().is_some_and(|d| !d.carried_out)
    }
}

pub(super) fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

pub(super) fn month_of(at: f64) -> String {
    chrono::DateTime::from_timestamp(at as i64, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m").to_string())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct Head {
    pub(super) hash: String,
    pub(super) rows: u64,
    /// The month the current file belongs to.
    pub(super) month: String,
}

pub(super) fn read_head(data_dir: &Path) -> Option<Head> {
    std::fs::read_to_string(data_dir.join(HEAD)).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// The last linked row's hash, read from the file itself — the fallback when the
/// head is missing, so a deleted `audit.head` does not restart the chain from
/// nothing (which would read as a break at the next row).
pub(super) fn tail_hash(data_dir: &Path) -> String {
    let text = std::fs::read_to_string(path(data_dir)).unwrap_or_default();
    text.lines().rev().find_map(|l| split_hash(l).map(|(_, h)| h.to_string())).unwrap_or_default()
}

/// A written line, split into the part that was hashed and the hash itself.
///
/// Works on the text rather than on a parsed `Entry` so a row written by a newer
/// build, with fields this one does not know, still verifies: re-serialising a
/// parsed row would silently drop them and report a break that is not there.
pub(super) fn split_hash(line: &str) -> Option<(String, &str)> {
    const KEY: &str = ",\"hash\":\"";
    let i = line.rfind(KEY)?;
    let rest = &line[i + KEY.len()..];
    let hash = rest.strip_suffix("\"}")?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((format!("{}}}", &line[..i]), hash))
}

pub(super) fn digest(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Months already rotated out, oldest first.
pub(super) fn rotated(data_dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(data_dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("audit-") && n.ends_with(".jsonl"))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Append one row, linked to the one before it. Never panics or blocks the caller on
/// failure — an audit write must not break the action it records.
pub fn append(data_dir: &Path, mut entry: Entry) {
    let Ok(_guard) = WRITER.lock() else { return };
    if entry.at == 0.0 {
        entry.at = now();
    }
    entry.detail = scrub(&entry.detail).chars().take(500).collect();
    entry.target = scrub(&entry.target).chars().take(300).collect();
    if let Some(s) = entry.subject.as_mut() {
        s.command = scrub(&s.command).chars().take(2000).collect();
        s.target = scrub(&s.target).chars().take(300).collect();
    }

    let month = month_of(entry.at);
    let mut head = read_head(data_dir).unwrap_or_else(|| Head { hash: tail_hash(data_dir), rows: 0, month: String::new() });
    // A new month starts a new file. The chain runs straight across: the first row
    // of September links to the last row of August.
    if !head.month.is_empty() && head.month != month && path(data_dir).exists() {
        let _ = std::fs::rename(path(data_dir), data_dir.join(format!("audit-{}.jsonl", head.month)));
    }
    head.month = month;

    entry.prev = head.hash.clone();
    entry.hash = String::new();
    let Ok(unlinked) = serde_json::to_string(&entry) else { return };
    let hash = digest(&unlinked);
    let line = format!("{},\"hash\":\"{hash}\"}}", &unlinked[..unlinked.len() - 1]);
    let _ = std::fs::create_dir_all(data_dir);
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(data_dir))
        .and_then(|mut f| writeln!(f, "{line}"));
    if written.is_ok() {
        head.hash = hash;
        head.rows += 1;
        if let Ok(json) = serde_json::to_string(&head) {
            let _ = crate::core::store::write_atomic(&data_dir.join(HEAD), &json);
        }
    }
}

/// Append one entry. Never panics or blocks the caller on failure — an audit
/// write must not break the action it records.
///
/// The first version's signature, kept for the thirty-odd places that record an
/// event with no decision behind it. The row is linked like any other.
pub fn log(data_dir: &Path, kind: &str, actor: &str, target: &str, detail: &str, ok: bool) {
    append(
        data_dir,
        Entry {
            kind: kind.to_string(),
            actor: actor.to_string(),
            target: target.to_string(),
            detail: detail.to_string(),
            ok,
            ..Default::default()
        },
    );
}

/// Every row, newest first, across the rotated months.
pub(super) fn rows_newest_first(data_dir: &Path) -> impl Iterator<Item = Entry> {
    let mut files = rotated(data_dir);
    files.push(path(data_dir));
    files.reverse();
    files.into_iter().flat_map(|f| {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let mut rows: Vec<Entry> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        rows.reverse();
        rows
    })
}

/// The most recent entries, newest first.
pub fn recent(data_dir: &Path, limit: usize) -> Vec<Entry> {
    rows_newest_first(data_dir).take(limit).collect()
}

/// Which rows a reader wants.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub limit: usize,
    pub session: Option<String>,
    pub persona: Option<String>,
    /// An event name or a family: `action` matches every `action.*`.
    pub event: Option<String>,
    /// Only runs nobody was watching — see [`Initiator::unwatched`].
    pub nobody_watching: bool,
    /// Only actions the boundaries refused.
    pub refused: bool,
    /// Only rows carrying a decision — the ones a rule can be replayed over.
    pub decisions: bool,
    /// Only rows written at or after this moment (seconds since the epoch). A session
    /// id is reused by the scoreboard across runs, so "this turn's rows" needs a time.
    pub since: f64,
}
