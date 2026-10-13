use super::*;

pub fn query(data_dir: &Path, f: &Filter) -> Vec<Entry> {
    let limit = if f.limit == 0 { 200 } else { f.limit };
    rows_newest_first(data_dir)
        .filter(|e| f.session.as_deref().is_none_or(|s| e.session == s || (e.session.is_empty() && e.actor == s)))
        .filter(|e| f.persona.as_deref().is_none_or(|p| e.persona == p))
        .filter(|e| {
            f.event.as_deref().is_none_or(|want| {
                let name = e.event_name();
                name == want || name.starts_with(&format!("{want}."))
            })
        })
        .filter(|e| !f.nobody_watching || e.initiator.as_ref().is_some_and(|i| i.unwatched()))
        .filter(|e| !f.refused || e.refused())
        .filter(|e| !f.decisions || e.decision.is_some())
        .filter(|e| e.at >= f.since)
        .take(limit)
        .collect()
}

/// What [`verify`] found.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Integrity {
    /// Rows read, linked or not.
    pub rows: u64,
    /// Rows carrying a link.
    pub linked: u64,
    /// Rows from before the trail was linked.
    pub legacy: u64,
    pub intact: bool,
    /// Where it first stops adding up, in words.
    pub problem: Option<String>,
}

/// Walk the chain from the oldest kept row to the newest.
pub fn verify(data_dir: &Path) -> Integrity {
    let _guard = WRITER.lock();
    let mut out = Integrity { intact: true, ..Default::default() };
    let mut files = rotated(data_dir);
    files.push(path(data_dir));
    let mut last = String::new();
    let mut anchored = false;
    for f in files {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or(FILE).to_string();
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            out.rows += 1;
            let Some((unlinked, hash)) = split_hash(line) else {
                if anchored {
                    out.intact = false;
                    out.problem.get_or_insert(format!(
                        "Row {} of {name} carries no link, but it comes after rows that do — it was added by something other than this app.",
                        i + 1
                    ));
                } else {
                    out.legacy += 1;
                }
                continue;
            };
            out.linked += 1;
            if digest(&unlinked) != hash {
                out.intact = false;
                out.problem.get_or_insert(format!("Row {} of {name} was changed after it was written.", i + 1));
            }
            let prev = serde_json::from_str::<Entry>(line).map(|e| e.prev).unwrap_or_default();
            // The oldest kept row is the anchor: what it links to may have been
            // removed by retention, which is a setting and says so in its own row.
            if anchored && prev != last {
                out.intact = false;
                out.problem.get_or_insert(format!(
                    "Row {} of {name} doesn't follow the row before it — something between them was removed or reordered.",
                    i + 1
                ));
            }
            anchored = true;
            last = hash.to_string();
        }
    }
    if let Some(head) = read_head(data_dir) {
        if anchored && head.hash != last {
            out.intact = false;
            out.problem.get_or_insert(
                "The newest rows are missing — the trail ends before the last row this app wrote.".to_string(),
            );
        }
    }
    out
}

/// Remove rotated months older than `keep_days`. Zero keeps everything, which is
/// the default. The removal is itself recorded, so a shorter trail says why.
pub fn apply_retention(data_dir: &Path, keep_days: u32) -> usize {
    if keep_days == 0 {
        return 0;
    }
    let cutoff = now() - keep_days as f64 * 86_400.0;
    let mut removed_rows = 0usize;
    let mut removed_files = 0usize;
    for f in rotated(data_dir) {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let newest = text.lines().rev().find_map(|l| serde_json::from_str::<Entry>(l).ok()).map(|e| e.at).unwrap_or(0.0);
        if newest < cutoff && std::fs::remove_file(&f).is_ok() {
            removed_rows += text.lines().filter(|l| !l.trim().is_empty()).count();
            removed_files += 1;
        }
    }
    if removed_files > 0 {
        let mut e = Entry::event("audit.retention");
        e.actor = "user".into();
        e.detail = format!(
            "Removed {removed_rows} rows from {removed_files} month(s) older than {keep_days} days, as the retention setting asks."
        );
        append(data_dir, e);
    }
    removed_rows
}

/// Erase the log (a user action — the panic-wipe path uses it too).
///
/// It is the user's machine and the user's trail, so this stays. What changes is
/// that the new trail opens by saying it happened, rather than looking like a
/// machine that has simply never done anything.
pub fn clear(data_dir: &Path) -> bool {
    let existed = {
        let _guard = WRITER.lock();
        let mut any = std::fs::remove_file(path(data_dir)).is_ok();
        for f in rotated(data_dir) {
            any |= std::fs::remove_file(f).is_ok();
        }
        let _ = std::fs::remove_file(data_dir.join(HEAD));
        any
    };
    if existed {
        let mut e = Entry::event("audit.wiped");
        e.actor = "user".into();
        e.detail = "The audit trail before this point was erased on this machine.".into();
        append(data_dir, e);
    }
    existed
}
