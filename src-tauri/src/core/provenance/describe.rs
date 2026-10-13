use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// Where it came from: a file path, a URL, a message id.
    pub source: String,
    /// What kind of thing that is — "file", "email", "web", "connector".
    #[serde(default)]
    pub kind: String,
    /// The line the agent read it from, when there is one worth quoting back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    /// Unix seconds.
    #[serde(default)]
    pub at: f64,
}

/// Sources for one workbook, keyed by "Sheet!A1".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default)]
    pub cells: BTreeMap<String, Source>,
}

/// Beside the workbook, not inside it: `budget.xlsx` → `budget.xlsx.sources.json`.
pub fn ledger_path(workbook: &Path) -> PathBuf {
    let mut name = workbook.file_name().unwrap_or_default().to_os_string();
    name.push(".sources.json");
    workbook.with_file_name(name)
}

/// The key a cell is recorded under. Normalised so "sheet1!b7" and "Sheet1!B7"
/// are the same cell — a citation nobody can look up is worse than none.
pub fn key(sheet: &str, cell: &str) -> String {
    format!("{}!{}", sheet.trim().to_lowercase(), cell.trim().to_uppercase())
}

pub fn load(workbook: &Path) -> Ledger {
    std::fs::read_to_string(ledger_path(workbook))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(workbook: &Path, l: &Ledger) -> Result<(), String> {
    crate::core::store::write_json_atomic(&ledger_path(workbook), l)
        .map_err(|e| format!("couldn't record sources: {e}"))
}

/// Record where some cells came from, keeping what was already there.
pub fn record(workbook: &Path, sheet: &str, entries: &[(String, Source)]) -> Result<usize, String> {
    let mut l = load(workbook);
    for (cell, src) in entries {
        l.cells.insert(key(sheet, cell), src.clone());
    }
    save(workbook, &l)?;
    Ok(entries.len())
}

/// Forget the sources for cells that are no longer there.
///
/// A citation pointing at a cell somebody deleted is a claim about a value that
/// does not exist, which is worse than an empty column.
pub fn prune(workbook: &Path, live: &[String]) -> Result<usize, String> {
    let mut l = load(workbook);
    let before = l.cells.len();
    let keep: std::collections::BTreeSet<String> = live.iter().cloned().collect();
    l.cells.retain(|k, _| keep.contains(k));
    let dropped = before - l.cells.len();
    if dropped > 0 {
        save(workbook, &l)?;
    }
    Ok(dropped)
}

/// A human-readable account of where a sheet's values came from.
pub fn describe(l: &Ledger, limit: usize) -> String {
    if l.cells.is_empty() {
        return "No sources recorded for this workbook.".into();
    }
    let mut out = String::new();
    for (k, s) in l.cells.iter().take(limit.max(1)) {
        let cell = k.split('!').next_back().unwrap_or(k);
        out.push_str(&format!("{cell}: {}", s.source));
        if let Some(q) = &s.quote {
            out.push_str(&format!(" — \"{}\"", q.chars().take(80).collect::<String>()));
        }
        out.push('\n');
    }
    if l.cells.len() > limit {
        out.push_str(&format!("…and {} more\n", l.cells.len() - limit));
    }
    out
}
