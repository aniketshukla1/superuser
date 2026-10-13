use super::*;

/// One file per run, inside the run's own folder so it is deleted with the
/// session and nothing has to remember to clean it up separately.
pub const FILE: &str = "run.jsonl";

/// Past this the file stops growing. A runaway turn should cost a truncated
/// journal, not a full disk; readers are told when it happened.
pub(super) const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Frame kinds that carry no information the final message doesn't: the token
/// stream. Everything else — tool calls, approvals, errors, notices — is kept.
pub(super) fn is_noise(kind: &str) -> bool {
    matches!(kind, "assistant_delta" | "reasoning_delta")
}

pub fn path(run_dir: &Path) -> PathBuf {
    run_dir.join(FILE)
}

/// Begin a run's journal, discarding the previous turn's.
///
/// One turn per session at a time, and the interesting question is always "what
/// did the last run do" — keeping every turn a session ever ran would grow without
/// bound for no one's benefit. The transcript is the durable record of the
/// conversation; this is the durable record of the working.
pub fn start(run_dir: &Path) {
    let _ = std::fs::create_dir_all(run_dir);
    let _ = std::fs::remove_file(path(run_dir));
}

/// Append one frame. Silent on every failure by design.
pub fn append(run_dir: &Path, frame: &Value) {
    let kind = frame["type"].as_str().unwrap_or("");
    if kind.is_empty() || is_noise(kind) {
        return;
    }
    let p = path(run_dir);
    if std::fs::metadata(&p).map(|m| m.len() >= MAX_BYTES).unwrap_or(false) {
        return;
    }
    // Serialize first, then scrub the whole line: a credential can arrive in any
    // field of any frame, and scrubbing per-field would mean knowing them all.
    let Ok(line) = serde_json::to_string(&json!({
        "at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0),
        "type": kind,
        "data": frame.get("data").cloned().unwrap_or(Value::Null),
    })) else {
        return;
    };
    let safe = crate::core::privacy::redact_secrets(&line);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{safe}");
    }
}

/// What a reader gets back.
pub struct Page {
    pub events: Vec<Value>,
    /// Offset to ask for next time — where this page ended.
    pub next_offset: usize,
    /// True when the run hit the size ceiling and stopped recording.
    pub truncated: bool,
}

/// Read a page of frames from `offset`, each stamped with its sequence number.
///
/// Reading line-by-line rather than slurping: a journal can be megabytes and a
/// caller usually wants the tail of it.
pub fn read(run_dir: &Path, offset: usize, limit: usize) -> Page {
    use std::io::{BufRead, BufReader};
    let p = path(run_dir);
    let truncated = std::fs::metadata(&p).map(|m| m.len() >= MAX_BYTES).unwrap_or(false);
    let Ok(file) = std::fs::File::open(&p) else {
        return Page { events: Vec::new(), next_offset: offset, truncated: false };
    };
    let mut events = Vec::new();
    let mut seq = 0usize;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        if seq >= offset && events.len() < limit {
            if let Ok(mut v) = serde_json::from_str::<Value>(&line) {
                v["seq"] = json!(seq);
                events.push(v);
            }
        }
        seq += 1;
    }
    let next_offset = offset + events.len();
    Page { events, next_offset, truncated }
}

/// How many frames a run recorded, without reading them.
pub fn len(run_dir: &Path) -> usize {
    use std::io::{BufRead, BufReader};
    std::fs::File::open(path(run_dir))
        .map(|f| BufReader::new(f).lines().map_while(Result::ok).filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}
