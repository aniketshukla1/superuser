use super::*;

/// Where a session's working folder lives — path only, nothing created. Use this
/// for reads; creating on read would leave an empty folder per session the UI
/// merely looks at.
/// Where a conversation's deliverables live.
///
/// Was a pure function of `data_dir` + `session_id`, recomputed on every call. That
/// is why the Files card's scratch-location setting could not be honoured: the card
/// promises "existing conversations keep their current folder", and a derived path
/// moves every one of them the moment the setting changes, orphaning the files they
/// already hold. So a session now RECORDS its folder the first time anything asks,
/// and this consults that record; `sessions::SessionStore::working_dir` owns the
/// pinning rule. Sessions with no record — a subagent's derived id, a run that has
/// not saved yet — fall through to the computed path exactly as before.
pub fn session_dir_path(data_dir: &Path, session_id: &str) -> PathBuf {
    let legacy = legacy_session_dir(data_dir, session_id);
    let base = crate::core::settings::Settings::load(&data_dir.to_path_buf()).scratch_base;
    crate::core::sessions::SessionStore::new(data_dir).working_dir(session_id, &base, legacy)
}

/// The original naming: `<data_dir>/sessions/<safe-id>`. Still the answer for
/// anything without a session record, and still what an existing conversation pins.
pub fn legacy_session_dir(data_dir: &Path, session_id: &str) -> PathBuf {
    let safe: String = session_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    data_dir.join("sessions").join(if safe.is_empty() { "session".into() } else { safe })
}

/// A per-session working folder under the app data dir (where deliverables land),
/// created if absent. For runs that are about to write.
pub fn session_dir(data_dir: &Path, session_id: &str) -> PathBuf {
    let dir = session_dir_path(data_dir, session_id);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Files present in the working folder — the artifacts the run produced.
pub fn list_artifacts(dir: &Path) -> Vec<String> {
    artifacts_since(dir, None)
}

/// What a turn produced, as paths relative to its working folder.
///
/// `since` is the moment the run began. Pass it whenever the folder is not the
/// run's own private one: with `None` this lists everything present, which is
/// exactly right for a per-run sandbox — everything in there IS the turn's output —
/// and catastrophic anywhere else. A run pointed at a real project reported 5,123
/// artifacts, `.git/objects` included, and wrote a 332KB list into its run record.
///
/// Directories the answer is never in are skipped outright rather than filtered
/// afterwards, so a large repository doesn't cost a full walk of its object store.
pub fn artifacts_since(dir: &Path, since: Option<std::time::SystemTime>) -> Vec<String> {
    const SKIP: &[&str] = &[".git", "node_modules", "target", ".venv", "dist", "build", ".next", "__pycache__"];
    let mut out = Vec::new();
    let walk = walkdir::WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().map(|n| SKIP.contains(&n)).unwrap_or(false));
    for e in walk.flatten() {
        if !e.file_type().is_file() {
            continue;
        }
        if let Some(t) = since {
            // Modified during the run, or it was already there and is not news.
            let touched = e.metadata().ok().and_then(|m| m.modified().ok()).map(|m| m >= t).unwrap_or(false);
            if !touched {
                continue;
            }
        }
        if let Ok(rel) = e.path().strip_prefix(dir) {
            out.push(rel.to_string_lossy().into_owned());
        }
    }
    out.sort();
    // A bound, because "what did this turn produce" is a list a person reads.
    out.truncate(200);
    out
}

/// Why `write_file` should stand aside for this path, if it should.
///
/// These extensions name container formats — a `.xlsx` is a zip archive, a `.docx`
/// is a zip of XML parts. Bytes written plainly under those names produce a file no
/// application will open, and `write_file` would report success anyway. `None`
/// means the path is ordinary text and writes normally.
///
/// Reading these formats works and is handled in the `read_file` arm; this is only
/// about *creating* them.
pub(super) fn document_write_refusal(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let use_instead = match ext.as_str() {
        "xlsx" | "xlsm" | "xls" | "ods" => "`sheet_new` to create it, then `sheet_write` for the cells",
        "docx" | "doc" | "odt" => "`doc_write`",
        "pptx" | "ppt" | "odp" => "`slides_write`",
        // No PDF writer exists here. Point at what does work rather than at a tool
        // the model will then fail to call.
        "pdf" => "`doc_write` for a .docx, or write the text as .md — there is no PDF writer",
        _ => return None,
    };
    Some(format!(
        "error: .{ext} is a binary document format — writing text to that name makes a \
         file no application can open. Use {use_instead}."
    ))
}
