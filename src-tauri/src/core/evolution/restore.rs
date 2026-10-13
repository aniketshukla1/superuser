use super::*;

/// Exactly what a review may write, and therefore exactly what is snapshotted.
///
/// Precision matters more here than convenience. Snapshotting the whole journal
/// would mean an undo also reverted a daily entry written after the pass (by an
/// unrelated conversation) and deleted the pass's own record — leaving no trace
/// that anything had happened.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Directories walked in full.
    pub dirs: Vec<PathBuf>,
    /// Individual files, which may live in directories that are otherwise off-limits.
    pub files: Vec<PathBuf>,
}

impl Scope {
    /// The skills and the core memory — and deliberately **not** the wiki.
    ///
    /// [`crate::core::wiki`] lives under `journal/wiki/` precisely so that it is
    /// absent from here. An undo means "that edit was wrong", and the fact that it
    /// was wrong is the single most useful thing a later pass could know; reverting
    /// the record of it alongside the edit is how the same rejected change gets
    /// proposed again next week. Skills roll back. Knowledge does not.
    pub fn for_review(data_dir: &Path, vault: &Path) -> Self {
        Self { dirs: vec![data_dir.join("skills")], files: vec![Journal::new(vault).core_path()] }
    }
}

/// A file as it was before the pass. `None` means it did not exist — restoring
/// then means deleting it, so a created file is undone as cleanly as an edited one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub taken_at: f64,
    pub files: BTreeMap<String, Option<String>>,
}

pub(super) fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Read every markdown file under the roots. These are small, hand-sized trees
/// (a few skills, a few weeks of journal), so a full copy is cheaper and far more
/// obviously correct than a diff.
pub fn snapshot(scope: &Scope) -> Snapshot {
    let mut files = BTreeMap::new();
    for root in &scope.dirs {
        for e in walkdir::WalkDir::new(root).max_depth(4).into_iter().flatten() {
            if e.file_type().is_file() && e.path().extension().map(|x| x == "md").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(e.path()) {
                    files.insert(e.path().to_string_lossy().into_owned(), Some(text));
                }
            }
        }
    }
    for f in &scope.files {
        // Recorded even when absent, so a file the pass creates is undone by
        // deleting it rather than left behind.
        files.insert(f.to_string_lossy().into_owned(), std::fs::read_to_string(f).ok());
    }
    let n: u64 = rand::random();
    Snapshot { id: format!("evo-{:010x}", n & 0xff_ffff_ffff), taken_at: now(), files }
}

/// Paths that differ from the snapshot — created, edited or deleted.
pub fn changed_since(snap: &Snapshot, scope: &Scope) -> Vec<String> {
    let after = snapshot(scope);
    let mut out: Vec<String> = Vec::new();
    for (path, before) in &after.files {
        if snap.files.get(path).and_then(|v| v.as_ref()) != before.as_ref() {
            out.push(path.clone());
        }
    }
    for path in snap.files.keys() {
        if !after.files.contains_key(path) {
            out.push(path.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Put the files back exactly as they were: rewrite what changed, delete what the
/// pass created. Returns how many files it restored.
pub fn restore(snap: &Snapshot, scope: &Scope) -> usize {
    let mut n = 0;
    for path in changed_since(snap, scope) {
        match snap.files.get(&path).and_then(|v| v.clone()) {
            Some(before) => {
                if let Some(dir) = Path::new(&path).parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if crate::core::store::write_atomic(Path::new(&path), &before).is_ok() {
                    n += 1;
                }
            }
            None => {
                if std::fs::remove_file(&path).is_ok() {
                    n += 1;
                }
            }
        }
    }
    n
}

pub(super) fn snapshot_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("evolution")
}

pub fn save_snapshot(data_dir: &Path, snap: &Snapshot) {
    let dir = snapshot_dir(data_dir);
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(s) = serde_json::to_string(snap) {
        let _ = crate::core::store::write_atomic(&dir.join(format!("{}.json", snap.id)), &s);
    }
}

/// Drop a snapshot once it has been used. Without this, undoing twice would walk
/// back through older passes, which is not what "undo that" means.
pub fn forget_snapshot(data_dir: &Path, id: &str) {
    let _ = std::fs::remove_file(snapshot_dir(data_dir).join(format!("{id}.json")));
}

/// The most recent pass's snapshot — what "undo the last change" restores.
pub fn latest_snapshot(data_dir: &Path) -> Option<Snapshot> {
    let mut snaps: Vec<Snapshot> = std::fs::read_dir(snapshot_dir(data_dir))
        .ok()?
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str::<Snapshot>(&s).ok())
        .collect();
    snaps.sort_by(|a, b| b.taken_at.partial_cmp(&a.taken_at).unwrap_or(std::cmp::Ordering::Equal));
    snaps.into_iter().next()
}
