use super::*;

/// A curated topic page.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Page {
    /// Filename stem — stable identity, derived from the title.
    pub slug: String,
    pub title: String,
    /// Free-form grouping the agent picks (`concepts`, `people`, `projects`, …).
    pub category: String,
    pub body: String,
    pub updated_at: String,
    /// `[[Other page]]` references found in the body.
    pub links: Vec<String>,
}

pub fn slugify(title: &str) -> String {
    let s: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "page".into() } else { s.chars().take(80).collect() }
}

/// `[[Wiki style]]` references, in order, deduplicated.
pub fn wikilinks(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        // `[[Page|label]]` — the target is the part before the pipe.
        let target = after[..end].split('|').next().unwrap_or("").trim().to_string();
        if !target.is_empty() && !out.contains(&target) {
            out.push(target);
        }
        rest = &after[end + 2..];
    }
    out
}

/// Drop a leading `# Heading` from a body. This module owns the title line, and a
/// model asked to write "the page" naturally opens with the title — leaving both in
/// gives every page a doubled heading and makes the index summary read `# Title`.
pub(super) fn strip_leading_h1(body: &str) -> String {
    let b = body.trim_start();
    match b.strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map(|(_, r)| r.trim_start()).unwrap_or("").trim_end().to_string(),
        None => b.trim_end().to_string(),
    }
}

/// The one-line gist shown in the index: the first line that is actual prose.
pub(super) fn summary_line(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("_"))
        .unwrap_or("")
        .chars()
        .take(120)
        .collect()
}

pub struct Knowledge {
    pub root: PathBuf,
}
