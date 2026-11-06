//! The shortlist: everyone the Talent Scout has put forward, so a later search leaves them out.
//!
//! `<data>/talent/shortlist.csv` holds when, for which role, and what recognises each person —
//! nothing else about anyone. It's written private to this account, and can be forgotten whole
//! from Settings.

use super::*;

const COLUMNS: [&str; 5] = ["date", "role", "name", "emails", "profiles"];

/// One person put forward for one role.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub date: String,
    pub role: String,
    pub person: Person,
}

pub fn shortlist_path(data_dir: &Path) -> PathBuf {
    data_dir.join("talent").join("shortlist.csv")
}

/// Everyone shortlisted so far, oldest first. A ledger that can't be read reads as empty.
pub fn load_shortlist(data_dir: &Path) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(shortlist_path(data_dir)) else { return Vec::new() };
    crate::core::expenses::csv_rows(&text)
        .into_iter()
        .skip(1)
        .filter(|f| f.len() >= COLUMNS.len())
        .map(|f| Entry {
            date: f[0].clone(),
            role: f[1].clone(),
            person: Person { name: f[2].clone(), emails: split_values(&f[3]), profiles: split_values(&f[4]) },
        })
        .collect()
}

/// Add `entries` to the ledger.
pub fn append_shortlist(data_dir: &Path, entries: &[Entry]) -> Result<(), String> {
    let path = shortlist_path(data_dir);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.trim().is_empty() {
        text = format!("{}\n", COLUMNS.join(","));
    } else if !text.ends_with('\n') {
        text.push('\n');
    }
    for e in entries {
        let (emails, profiles) = (e.person.emails.join(" "), e.person.profiles.join(" "));
        text.push_str(&crate::core::expenses::csv_line(&[&e.date, &e.role, &e.person.name, &emails, &profiles]));
        text.push('\n');
    }
    std::fs::create_dir_all(path.parent().unwrap_or(data_dir)).map_err(|e| format!("couldn't make the talent folder: {e}"))?;
    crate::core::store::write_atomic_private(&path, &text).map_err(|e| format!("couldn't write the shortlist: {e}"))
}

/// Forget everyone on the shortlist, returning how many there were.
pub fn forget_shortlist(data_dir: &Path) -> Result<usize, String> {
    let n = load_shortlist(data_dir).len();
    match std::fs::remove_file(shortlist_path(data_dir)) {
        Ok(()) => Ok(n),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(format!("couldn't remove the shortlist: {e}")),
    }
}
