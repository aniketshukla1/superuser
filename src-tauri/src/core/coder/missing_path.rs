use super::*;

pub(super) const SKIP: &[&str] = &[
    "node_modules", ".git", "target", "dist", "build", ".next", ".venv",
    "__pycache__", ".cache", ".idea", "venv", "out", ".turbo",
];

pub(super) const MAX_READ: usize = 400_000;

/// Resolve `path` and ensure it lives inside `root` (blocks `..` escapes).
pub(super) fn within(root: &str, path: &str) -> Result<PathBuf, String> {
    let root_c = Path::new(root).canonicalize().map_err(|e| format!("bad workspace: {e}"))?;
    let p_c = Path::new(path).canonicalize().map_err(|_| missing_path(&root_c, path))?;
    if p_c.starts_with(&root_c) {
        Ok(p_c)
    } else {
        Err("path is outside the workspace".into())
    }
}

/// The error for a path that is not there, written so the next step is obvious.
///
/// `error: no such path: No such file or directory (os error 2)` was the whole
/// message, and it is a dead end: it says the guess was wrong and nothing about what
/// would have been right. Measured on this machine's own session logs
/// (TOOLS-RESEARCH-2026 §11): `read_file` was called nine times and failed nine
/// times, every one of them a path the model invented — while `list_dir` was called
/// once and `grep` never, across 59 sessions. The model guesses instead of looking,
/// and the tool that catches the guess says nothing to change that.
///
/// So the error names the deepest directory on the guessed path that **does** exist
/// and shows what is in it. That is the same shape as `str_replace`'s multi-match
/// error and the truncation notice that made `recall_output` work: state the fact,
/// then name the move.
pub(super) fn missing_path(root_c: &Path, guessed: &str) -> String {
    // Walk up from the guess to the first directory that exists, staying inside root.
    let mut probe: PathBuf = if Path::new(guessed).is_absolute() {
        PathBuf::from(guessed)
    } else {
        root_c.join(guessed)
    };
    let mut climbed = false;
    while !probe.is_dir() {
        match probe.parent() {
            Some(parent) if parent.starts_with(root_c) || parent == root_c => {
                probe = parent.to_path_buf();
                climbed = true;
            }
            _ => {
                probe = root_c.to_path_buf();
                climbed = true;
                break;
            }
        }
    }
    let shown = probe.strip_prefix(root_c).ok().map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
    let here = if shown.is_empty() { ".".to_string() } else { shown };

    let mut names: Vec<String> = std::fs::read_dir(&probe)
        .map(|rd| {
            rd.flatten()
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .map(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() { format!("{n}/") } else { n }
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    // Enough to recognise the right one, not enough to be a directory dump — this
    // lands in the context window of every wrong guess.
    let listed = if names.is_empty() {
        "it is empty".to_string()
    } else {
        let head: Vec<String> = names.iter().take(12).cloned().collect();
        let more = names.len().saturating_sub(head.len());
        format!("it holds: {}{}", head.join(", "), if more > 0 { format!(", and {more} more") } else { String::new() })
    };
    // The clause added after this error caused a regression, which is the honest way
    // to describe it. `admits_a_missing_file` went 3/3 to 1/3 on the sweep that
    // introduced the teaching version, and the failing answer said why: *"Running
    // `list_dir` to find the actual file path…"*. That task is about a file the user
    // named that is not there, where "it does not exist" IS the answer — and this
    // error had just told the model, unconditionally, to go and hunt.
    //
    // It now offers both moves and puts absence first. Guessing is still the failure
    // this exists to stop; a file that genuinely is not there is not a guess.
    format!(
        "no such path: {guessed}. `{here}` exists and {listed}. \
         If this is the file you were asked about, it is simply not there — say so. \
         If you picked this name yourself while looking for something, find the real one \
         with list_dir or grep rather than guessing again."
    )
}

/// Resolve a path that **doesn't exist yet** and ensure it lands inside `root`.
///
/// [`within`] can't do this job: `canonicalize` requires the file to be there, so
/// every create/write path used to skip it and check the *string* instead —
/// rejecting `..` and absolute paths and then joining onto the root. Two things
/// walk past a string check. A symlinked directory inside the workspace is a
/// perfectly ordinary relative path that resolves somewhere else entirely. And
/// `Path::join` **replaces** the base when the argument is absolute, so a check
/// that rejects `..` but not `/` isn't rejecting anything at all.
///
/// So: resolve the nearest ancestor that does exist — which is what collapses any
/// symlink in the path — and re-attach the remainder lexically, refusing `..` in
/// it rather than trying to normalise it away. A dangling symlink anywhere in the
/// path fails closed, because `canonicalize` can't resolve it and we don't guess.
pub(super) fn within_new(root: &str, path: &str) -> Result<PathBuf, String> {
    let root_c = Path::new(root).canonicalize().map_err(|e| format!("bad workspace: {e}"))?;
    let given = Path::new(path);
    if given.as_os_str().is_empty() {
        return Err("invalid path".into());
    }
    // An absolute path is never relative-to-root, whatever `join` would do with it.
    let target = if given.is_absolute() { given.to_path_buf() } else { root_c.join(given) };

    // `symlink_metadata` rather than `exists`: the latter follows symlinks, so a
    // dangling one would look absent and we'd happily plan a write through it.
    let mut existing = target.as_path();
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    while existing.symlink_metadata().is_err() {
        let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
            return Err("invalid path".into());
        };
        rest.push(name);
        existing = parent;
    }
    let mut full = existing.canonicalize().map_err(|e| format!("no such path: {e}"))?;
    for name in rest.iter().rev() {
        if *name == ".." || *name == "." {
            return Err("invalid path".into());
        }
        full.push(name);
    }
    if full.starts_with(&root_c) {
        Ok(full)
    } else {
        Err("path is outside the workspace".into())
    }
}

/// Where a write to `path` would actually land, or an error if that's outside
/// `root`. Public so the agent's own `write_file` tool resolves paths the same
/// way the Coder does — it previously had its own weaker string check.
pub fn resolve_write_path(root: &str, path: &str) -> Result<PathBuf, String> {
    within_new(root, path)
}

/// Read a text file inside the workspace (size-capped).
pub fn read(root: &str, path: &str) -> Result<String, String> {
    let p = within(root, path)?;
    let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
    let truncated = bytes.len() > MAX_READ;
    let slice = if truncated { &bytes[..MAX_READ] } else { &bytes[..] };
    let mut s = String::from_utf8_lossy(slice).to_string();
    if truncated {
        s.push_str("\n\n… (file truncated)");
    }
    Ok(s)
}

// Kept for the test suite only: the adversarial probes assert these reject path
// escapes and NUL bytes. Production writes go through `resolve_write_path`.
#[cfg(test)]
/// Create a NEW file (relative path) inside the workspace, making parent dirs.
/// Errors if it already exists or the path escapes the workspace.
pub fn create(root: &str, rel: &str, content: &str) -> Result<(), String> {
    if rel.trim().is_empty() || Path::new(rel).is_absolute() || rel.split(['/', '\\']).any(|c| c == "..") {
        return Err("invalid path".into());
    }
    // The string check above is kept for its clearer error, but it is not what
    // makes this safe — `within_new` is, because a symlinked parent is neither
    // absolute nor contains "..".
    let full = within_new(root, rel)?;
    if full.exists() {
        return Err("file already exists".into());
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full, content).map_err(|e| e.to_string())
}

// Kept for the test suite only: the adversarial probes assert these reject path
// escapes and NUL bytes. Production writes go through `resolve_write_path`.
#[cfg(test)]
/// Delete an existing file (relative path) inside the workspace.
pub fn remove(root: &str, rel: &str) -> Result<(), String> {
    let full = Path::new(root).join(rel);
    let p = within(root, &full.to_string_lossy())?; // must exist + be inside root
    if !p.is_file() {
        return Err("not a file".into());
    }
    std::fs::remove_file(&p).map_err(|e| e.to_string())
}
