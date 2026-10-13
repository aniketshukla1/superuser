use super::super::*;

/// One outbound request, as it will be shown on the Privacy page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Egress {
    pub at: f64,
    /// Who wanted it: a connector id, `model`, `web`, `mcp`, `update`.
    pub purpose: String,
    pub method: String,
    pub host: String,
    /// Path only. The query string is deliberately dropped — see the module docs.
    pub path: String,
    /// Request body size in bytes, 0 when there wasn't one.
    pub sent: u64,
    /// HTTP status, or `None` when the request never got that far.
    #[serde(default)]
    pub status: Option<u16>,
    /// Set when local-only mode refused it, or the request failed.
    #[serde(default)]
    pub error: Option<String>,
    /// True when local-only mode stopped this rather than the network.
    #[serde(default)]
    pub blocked: bool,
}

pub(in crate::core::egress) fn path_of(data_dir: &Path) -> PathBuf {
    data_dir.join("egress.jsonl")
}

/// Keep the log bounded. Outbound traffic is far higher volume than the action
/// log, and an always-on app must not grow a file forever; half a megabyte is
/// several thousand lines, which is more history than the page can show.
pub(in crate::core::egress) const MAX_BYTES: u64 = 512 * 1024;

/// Serializes writes to the log.
///
/// Rotation is a read-modify-write of the whole file, and the app makes outbound
/// requests from six background loops plus the user's turn — so an append lands in
/// the middle of a rotation as a matter of course, not as a rare race. Measured
/// before this lock existed: 24,000 concurrent records left **711 of 1,472**
/// retained lines unparseable. A torn line is a row the Privacy page drops without
/// saying so, which is the one failure a log like this must not have.
///
/// A mutex rather than something cleverer because the critical section is a
/// few microseconds of formatting and one small write, and correctness here is
/// worth more than concurrent logging ever could be.
pub(in crate::core::egress) fn write_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

pub(in crate::core::egress) fn record(e: Egress) {
    let Some(dir) = log_dir().lock().ok().and_then(|d| d.clone()) else { return };
    let p = path_of(&dir);
    let _guard = write_lock().lock().unwrap_or_else(|e| e.into_inner());

    if std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        // Drop the older half rather than the whole file: a log that empties itself
        // completely is one you can't trust to have been there a minute ago.
        //
        // Written to a temp file and renamed rather than truncated in place. The
        // lock already keeps our own threads out of each other's way; the rename
        // is what makes the swap atomic to anything *reading* the log, and what
        // stops a crash mid-rotation leaving a half-written file behind.
        if let Ok(text) = std::fs::read_to_string(&p) {
            let lines: Vec<&str> = text.lines().collect();
            let keep = lines[lines.len() / 2..].join("\n");
            let tmp = p.with_extension("jsonl.rotating");
            if std::fs::write(&tmp, format!("{keep}\n")).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }
    if let Ok(line) = serde_json::to_string(&e) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The policy and log location are process-wide by design — the code that has to
/// honour them doesn't hold `Settings`. Any test that calls [`set_data_dir`] or
/// [`set_local_only`] is therefore mutating shared state, so they take turns on
/// this. Lives outside `mod tests` because the probes in `crate::adversarial`
/// need the same lock; if they didn't take it, they'd redirect the log out from
/// under a test running in parallel.
///
/// Poisoning is ignored on purpose: one failing test should report its own
/// failure, not turn every other test into a panic about a mutex.
#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

/// Put a row in the log without a network round-trip, for the scale tests. The
/// real path is `RequestBuilder::send`; this is the same `record` it calls.
#[cfg(test)]
pub(crate) fn record_for_test(host: &str, purpose: &str, sent: u64) {
    record(Egress {
        at: now(),
        purpose: purpose.into(),
        method: "GET".into(),
        host: host.into(),
        path: "/v1/thing".into(),
        sent,
        status: Some(200),
        error: None,
        blocked: false,
    });
}

/// The most recent requests, newest first.
pub fn recent(data_dir: &Path, limit: usize) -> Vec<Egress> {
    let text = std::fs::read_to_string(path_of(data_dir)).unwrap_or_default();
    let mut out: Vec<Egress> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    out.reverse();
    out.truncate(limit);
    out
}

/// Everything that has been talked to, busiest first.
///
/// The rollup is the answer to the question people actually ask. A list of
/// requests says what happened; "these six hosts, this much data" says who has
/// heard from you, which is the thing a privacy page exists to tell you.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostTotal {
    pub host: String,
    pub requests: usize,
    pub sent: u64,
    pub blocked: usize,
    pub last: f64,
    /// Every purpose that has talked to this host.
    pub purposes: Vec<String>,
}

pub fn by_host(data_dir: &Path) -> Vec<HostTotal> {
    let mut totals: Vec<HostTotal> = Vec::new();
    let text = std::fs::read_to_string(path_of(data_dir)).unwrap_or_default();
    for e in text.lines().filter_map(|l| serde_json::from_str::<Egress>(l).ok()) {
        match totals.iter_mut().find(|t| t.host == e.host) {
            Some(t) => {
                t.requests += 1;
                t.sent += e.sent;
                t.blocked += usize::from(e.blocked);
                t.last = t.last.max(e.at);
                if !t.purposes.contains(&e.purpose) {
                    t.purposes.push(e.purpose);
                }
            }
            None => totals.push(HostTotal {
                host: e.host,
                requests: 1,
                sent: e.sent,
                blocked: usize::from(e.blocked),
                last: e.at,
                purposes: vec![e.purpose],
            }),
        }
    }
    totals.sort_by(|a, b| b.requests.cmp(&a.requests));
    totals
}

pub fn clear(data_dir: &Path) -> bool {
    std::fs::remove_file(path_of(data_dir)).is_ok()
}

/// Split a URL into the host and the path, dropping the query.
///
/// Returns `None` for anything unparseable, which the caller treats as remote —
/// failing closed here means an odd URL gets recorded and refused under
/// local-only, rather than slipping past both.
pub(in crate::core::egress) fn split_url(url: &str) -> Option<(String, String)> {
    let (_, after) = url.split_once("://")?;
    let (hostport, rest) = match after.find(['/', '?', '#']) {
        Some(i) => (&after[..i], &after[i..]),
        None => (after, "/"),
    };
    let hostport = hostport.rsplit('@').next().unwrap_or(hostport);
    let host = if let Some(r) = hostport.strip_prefix('[') {
        r.split(']').next().unwrap_or(r).to_string()
    } else {
        hostport.rsplit_once(':').map(|(h, _)| h).unwrap_or(hostport).to_string()
    };
    let path = rest.split(['?', '#']).next().unwrap_or("/");
    (!host.is_empty()).then(|| (host.to_lowercase(), path.to_string()))
}
