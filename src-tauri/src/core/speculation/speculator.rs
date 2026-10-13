use super::*;

/// How many files may be prefetched from one grep result.
///
/// One. The mined pattern is "the top hit", and a speculator that reads five files
/// on the chance one is wanted is a thundering herd wearing an optimisation's
/// clothes — the same reasoning that keeps `PARALLEL_READS` at four.
pub const PREFETCH_LIMIT: usize = 1;

/// Biggest file worth prefetching, in bytes.
///
/// A speculative read of a 40MB log spends real disk to maybe save a wait, and the
/// result would be truncated into a handle anyway (see `core::observations`).
pub const MAX_PREFETCH_BYTES: u64 = 512 * 1024;

/// The first file path named in a grep-style result.
///
/// Reads the shape our own `grep` emits — `path:line:text` — and stops at the first
/// line that has one, which is the top hit. Deliberately not a general parser: a
/// path this does not recognise means no speculation, which is the safe direction.
pub fn top_hit(result: &str) -> Option<String> {
    for line in result.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("error:") {
            continue;
        }
        // `path:line:text`. Take everything before the first colon that is followed
        // by digits — a bare `foo.rs:12:` is a hit, `http://x` is not.
        let mut parts = line.splitn(3, ':');
        let (Some(path), Some(num)) = (parts.next(), parts.next()) else {
            continue;
        };
        if path.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) || num.is_empty() {
            continue;
        }
        return Some(path.to_string());
    }
    None
}

/// Whether `rel` stays inside `root` once resolved.
///
/// Lexical, and on purpose: this runs before the file is opened, so it cannot rely
/// on canonicalising something that may not exist. `..` is rejected outright rather
/// than resolved, because a speculative read is not worth the one case where
/// clever path arithmetic lands somewhere surprising.
pub fn inside(root: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    Some(root.join(p))
}

/// One outstanding guess, and whether it paid off.
#[derive(Debug, Default)]
pub struct Speculator {
    /// The path we prefetched, relative to the run root, and what it held.
    pub(super) holding: Option<(String, String)>,
    pub(super) hits: usize,
    pub(super) misses: usize,
    pub(super) started: usize,
}

impl Speculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Consider prefetching from a completed call.
    ///
    /// `tainted` is the run's taint flag: a run that has read a stranger's words
    /// does not get to have reads made on its behalf, because the thing choosing
    /// what to prefetch is downstream of that text.
    pub fn consider(&mut self, root: &Path, tool: &str, result: &str, tainted: bool) {
        if tainted || self.holding.is_some() {
            return;
        }
        if !matches!(tool, "grep" | "find_in_files") {
            return;
        }
        let Some(rel) = top_hit(result) else { return };
        let Some(full) = inside(root, &rel) else { return };
        // Size first: a stat is cheap, and the point is to avoid spending the read.
        match std::fs::metadata(&full) {
            Ok(m) if m.is_file() && m.len() <= MAX_PREFETCH_BYTES => {}
            _ => return,
        }
        if let Ok(text) = std::fs::read_to_string(&full) {
            self.started += 1;
            self.holding = Some((rel, text));
        }
    }

    /// The prefetched contents, if this is the read we guessed.
    ///
    /// Consumes the guess either way: one prefetch answers at most one call, and a
    /// stale hold would serve a file that has since been written by the same turn.
    pub fn take(&mut self, tool: &str, path: &str) -> Option<String> {
        if tool != "read_file" {
            return None;
        }
        match self.holding.take() {
            Some((held, text)) if held == path => {
                self.hits += 1;
                Some(text)
            }
            Some(_) => {
                self.misses += 1;
                None
            }
            None => None,
        }
    }

    /// Drop any outstanding guess. Called when the workspace is written to, because
    /// a prefetch made before a write is a stale answer afterwards.
    pub fn invalidate(&mut self) {
        if self.holding.take().is_some() {
            self.misses += 1;
        }
    }

    pub fn hits(&self) -> usize {
        self.hits
    }

    pub fn misses(&self) -> usize {
        self.misses
    }

    pub fn started(&self) -> usize {
        self.started
    }

    /// A one-line summary for a trace.
    pub fn summary(&self) -> String {
        format!(
            "speculation: {} started, {} hit, {} missed",
            self.started, self.hits, self.misses
        )
    }
}
