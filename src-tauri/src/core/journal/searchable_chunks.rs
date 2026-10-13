//! Searchable chunks

use super::*;

/// Which tier a chunk came from. The distinction is not cosmetic: it decides
/// whether the chunk fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Distilled long-term memory. Survived a consolidation, so it does not decay —
    /// "prefers concise answers" is no less true for being six months old.
    Core,
    /// A dated conversation summary. Decays: what mattered in a conversation three
    /// months ago is usually less relevant than yesterday's, and without decay the
    /// oldest, most numerous entries would crowd out the recent ones.
    Daily,
}

/// Half-life for daily memory relevance. After 30 days a chunk scores half of what
/// it did when written; after 60, a quarter.
pub const DAILY_HALF_LIFE_DAYS: f64 = 30.0;

/// One searchable unit of the journal.
#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    /// Stable across runs so an embedding index can key on it: `core#2`,
    /// `daily/2026-08-12#14:20`.
    pub id: String,
    pub tier: Tier,
    /// ISO date for daily chunks; empty for core.
    pub date: String,
    pub text: String,
}

/// How much a daily chunk written on `date` still counts today, on a 30-day
/// half-life. Core memory never calls this. A date we can't parse is treated as
/// today rather than as infinitely old — a malformed filename should not silently
/// bury a real memory.
pub fn decay_factor(date: &str, today: chrono::NaiveDate) -> f64 {
    let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") else { return 1.0 };
    let days = (today - d).num_days().max(0) as f64;
    0.5f64.powf(days / DAILY_HALF_LIFE_DAYS)
}

/// Split a daily file into its timestamped sections. A daily file is a sequence of
/// `## HH:MM` blocks, each one summary of one stretch of conversation — the right
/// retrieval unit, since a whole day's file would dilute any single fact in it.
pub(super) fn split_daily(body: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut heading = String::new();
    let mut buf = String::new();
    for line in body.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            if !buf.trim().is_empty() {
                out.push((heading.clone(), buf.trim().to_string()));
            }
            heading = h.trim().to_string();
            buf.clear();
        } else if !line.starts_with("# ") {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if !buf.trim().is_empty() {
        out.push((heading, buf.trim().to_string()));
    }
    out
}

impl Journal {
    /// Every searchable unit: one chunk per core bullet, one per dated daily
    /// section. Core bullets are chunked individually because they are already
    /// single dense statements — embedding all thirty as one blob would make every
    /// query match everything equally.
    pub fn chunks(&self) -> Vec<Chunk> {
        let mut out = Vec::new();
        for (i, line) in self.read_core().lines().enumerate() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            out.push(Chunk { id: format!("core#{i}"), tier: Tier::Core, date: String::new(), text: t.trim_start_matches(['-', '*', ' ']).to_string() });
        }
        for e in self.dailies() {
            // Keyed by position, not by the `## HH:MM` heading: two summaries written
            // in the same minute share a heading, and a heading-keyed id would make
            // them collide — the embedding index would keep one and lose the other.
            // Daily files are append-only, so position is stable.
            for (i, (_, text)) in split_daily(&e.body).into_iter().enumerate() {
                out.push(Chunk {
                    id: format!("daily/{}#{i}", e.date),
                    tier: Tier::Daily,
                    date: e.date.clone(),
                    text,
                });
            }
        }
        out
    }
}

/// Keyword overlap, normalized to roughly 0..1 so it can be blended with cosine
/// similarity. Deliberately simple: it exists to catch the exact-term matches that
/// embeddings are bad at (a project name, an error code, a person), not to be a
/// search engine on its own.
pub fn keyword_score(query: &str, text: &str) -> f64 {
    let terms: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(|t| t.to_string())
        .collect();
    if terms.is_empty() {
        return 0.0;
    }
    let hay = text.to_lowercase();
    let hits = terms.iter().filter(|t| hay.contains(t.as_str())).count();
    hits as f64 / terms.len() as f64
}
