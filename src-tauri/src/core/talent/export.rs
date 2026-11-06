//! The ATS export: everyone the person's applicant tracking system already has.
//!
//! Every ATS exports candidates as CSV — Greenhouse, Lever, Ashby, Workable, a spreadsheet — and
//! each names its columns its own way, so they're found by their headers.

use super::*;

/// Which columns hold what, found by header.
#[derive(Debug, Default, PartialEq)]
struct Columns {
    full: Option<usize>,
    first: Option<usize>,
    last: Option<usize>,
    emails: Vec<usize>,
    profiles: Vec<usize>,
}

fn columns(header: &[String]) -> Columns {
    let mut c = Columns::default();
    for (i, h) in header.iter().enumerate() {
        let h = h.trim().trim_start_matches('\u{feff}').to_lowercase();
        let has = |w: &str| h.contains(w);
        if has("email") || has("e-mail") {
            c.emails.push(i);
        } else if ["link", "github", "gitlab", "url", "website", "profile", "portfolio"].iter().any(|w| has(w)) {
            c.profiles.push(i);
        } else if matches!(h.as_str(), "name" | "full name" | "candidate" | "candidate name") {
            c.full.get_or_insert(i);
        } else if has("first") && has("name") {
            c.first.get_or_insert(i);
        } else if (has("last") && has("name")) || has("surname") || has("family name") {
            c.last.get_or_insert(i);
        }
    }
    c
}

/// The people in an export, each with the row a spreadsheet shows it on (the header is row 1).
pub fn read_export(text: &str) -> Result<Vec<(usize, Person)>, String> {
    let rows = crate::core::expenses::csv_rows(text);
    let Some(header) = rows.first() else { return Err("it's empty".into()) };
    let c = columns(header);
    if c.emails.is_empty() && c.profiles.is_empty() && c.full.is_none() && c.first.is_none() {
        return Err("its first row has no name, email or profile column — export it as CSV with headers".into());
    }
    let cell = |r: &[String], i: usize| r.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
    Ok(rows
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(row, r)| {
            let full = c.full.map(|i| cell(r, i)).filter(|s| !s.is_empty());
            let name = full.unwrap_or_else(|| {
                format!("{} {}", c.first.map(|i| cell(r, i)).unwrap_or_default(), c.last.map(|i| cell(r, i)).unwrap_or_default())
                    .trim()
                    .to_string()
            });
            let p = Person {
                name,
                emails: c.emails.iter().flat_map(|&i| split_values(&cell(r, i))).collect(),
                profiles: c.profiles.iter().flat_map(|&i| split_values(&cell(r, i))).collect(),
            };
            (!p.name.is_empty() || !p.emails.is_empty() || !p.profiles.is_empty()).then_some((row + 1, p))
        })
        .collect())
}

/// `~/…` as the home folder: how people write paths.
pub fn expand_home(path: &str) -> PathBuf {
    match (path.trim().strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path.trim()),
    }
}

/// The export at `path`, read now: exporting again to the same place is how it's kept current.
/// Read leniently — a spreadsheet saved in an old encoding still has its emails in ASCII.
pub fn load_export(path: &str) -> Result<Vec<(usize, Person)>, String> {
    let full = expand_home(path);
    let bytes = std::fs::read(&full).map_err(|e| format!("couldn't read your ATS export at {}: {e}", full.display()))?;
    read_export(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("your ATS export at {}: {e}", full.display()))
}
