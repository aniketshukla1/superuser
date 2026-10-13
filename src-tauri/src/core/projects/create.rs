use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub folder: Option<String>,
    pub created_at: i64,
    /// When this folder was last opened, which is what "recent" is actually about.
    /// Absent on rows written before this field existed; `recency` falls back to
    /// `created_at` for those, so an old store still orders sensibly.
    #[serde(default)]
    pub last_opened: Option<i64>,
}

/// What "recent" means for one row.
pub(super) fn recency(p: &Project) -> i64 {
    p.last_opened.unwrap_or(p.created_at)
}

pub(super) fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("projects.json")
}

pub(super) fn load_all(data_dir: &Path) -> Vec<Project> {
    std::fs::read_to_string(store_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(super) fn save_all(data_dir: &Path, all: &[Project]) {
    if let Ok(s) = serde_json::to_string(all) {
        let _ = crate::core::store::write_atomic(&store_path(data_dir), &s);
    }
}

pub(super) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Most-recently-opened first, one row per folder.
///
/// Both halves of that sentence used to be false, and each was false in a way nothing
/// reported.
///
/// The sort was ASCENDING. `/v1/workspaces/recent` served its list oldest-first, and
/// `/v1/health` builds `default_workspace` from `list().first()` — so a cold boot with
/// no conversation to resume seeded a workbench with the oldest folder the user had
/// ever opened, and looked entirely deliberate doing it.
///
/// And rows were never distinct: `workspace_open` calls `create` on EVERY open, which
/// appended unconditionally. Opening the same folder four times wrote four identical
/// rows, so the folder gate's Recent list offered one folder four times and React
/// warned about the duplicate keys underneath it.
///
/// Deduping on READ rather than migrating the file: the duplicates are already written
/// on every install that has been used, and collapsing them here fixes those without
/// rewriting anyone's store to find out whether it worked.
pub fn list(data_dir: &Path) -> Vec<Project> {
    let mut all = load_all(data_dir);
    all.sort_by(|a, b| recency(b).cmp(&recency(a)).then_with(|| b.created_at.cmp(&a.created_at)));
    let mut seen = std::collections::HashSet::new();
    all.retain(|p| match p.folder.as_deref() {
        // A folderless project is a container, not a place — two of them are two
        // projects, however alike they look.
        None | Some("") => true,
        Some(f) => seen.insert(f.to_string()),
    });
    all
}

/// Create a project. If `folder` is a non-empty path that doesn't exist yet, we
/// create it (the user already approved the location via the OS dialog).
pub fn create(data_dir: &Path, name: &str, folder: Option<String>) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name can't be empty.".into());
    }
    let folder = folder.filter(|f| !f.trim().is_empty());
    if let Some(f) = &folder {
        if !Path::new(f).exists() {
            std::fs::create_dir_all(f).map_err(|e| format!("Couldn't create folder: {e}"))?;
        }
    }
    let mut all = load_all(data_dir);
    // Opening a folder you have opened before is the SAME project, not a new one. This
    // is called on every `workspace_open`, so appending unconditionally grew the store
    // by one identical row per open. The folder is the identity here; the name is not
    // touched, because renaming a project is `rename`'s job and doing it as a side
    // effect of opening a folder would be a surprise nobody asked for.
    if let Some(f) = &folder {
        if let Some(existing) = all.iter_mut().find(|p| p.folder.as_deref() == Some(f.as_str())) {
            existing.last_opened = Some(now_ms());
            let touched = existing.clone();
            save_all(data_dir, &all);
            return Ok(touched);
        }
    }
    let now = now_ms();
    let project = Project {
        id: format!("proj-{now}"),
        name: name.to_string(),
        folder,
        created_at: now,
        last_opened: Some(now),
    };
    all.push(project.clone());
    save_all(data_dir, &all);
    Ok(project)
}

pub fn rename(data_dir: &Path, id: &str, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let mut all = load_all(data_dir);
    if let Some(p) = all.iter_mut().find(|p| p.id == id) {
        p.name = name.to_string();
        save_all(data_dir, &all);
    }
}

pub fn delete(data_dir: &Path, id: &str) {
    let mut all = load_all(data_dir);
    all.retain(|p| p.id != id);
    save_all(data_dir, &all);
}
