//! Journal recall (hybrid: vector + keyword, decayed by tier)

use super::*;

/// Weight on semantic similarity when blending the two signals. The rest goes to
/// keyword overlap, which is there to catch the exact tokens embeddings smear over
/// — a project name, an error code, a person's name.
pub(super) const VECTOR_WEIGHT: f64 = 0.7;

pub(super) const KEYWORD_WEIGHT: f64 = 1.0 - VECTOR_WEIGHT;

/// Below this a chunk is noise rather than a weak match, and putting it in the
/// prompt costs tokens and invites the model to use something irrelevant.
pub(super) const JOURNAL_FLOOR: f64 = 0.22;

/// One journal chunk that matched, with the score it matched at.
#[derive(Debug, Clone, serde::Serialize)]
pub struct JournalHit {
    pub id: String,
    pub tier: crate::core::journal::Tier,
    pub date: String,
    pub text: String,
    pub score: f64,
}

pub(super) fn journal_index_path(data_dir: &Path) -> PathBuf {
    data_dir.join("journal-embeddings.json")
}

/// Search the memory journal. Blends semantic similarity with keyword overlap and
/// then fades daily memory on a 30-day half-life, so recent conversation summaries
/// outrank equally-similar old ones while distilled core memory never fades.
///
/// Degrades rather than fails: with no embedding model reachable, it still returns
/// keyword-ranked results (the blend simply loses its vector half), because a
/// memory you can't search is worse than one searched crudely.
pub async fn recall_journal(
    journal: &crate::core::journal::Journal,
    settings: &Settings,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> Vec<JournalHit> {
    use crate::core::journal::{decay_factor, keyword_score, Tier};

    if query.trim().is_empty() {
        return vec![];
    }
    let chunks = journal.chunks();
    if chunks.is_empty() {
        return vec![];
    }

    // Vector half — best effort. `vectors` stays empty if embedding is unavailable.
    let mut vectors: HashMap<String, Vec<f32>> = HashMap::new();
    let mut qvec: Vec<f32> = Vec::new();
    if let Ok(v) = journal_vectors(&chunks, settings, data_dir).await {
        vectors = v;
        qvec = embed(settings, vec![query.to_string()]).await.ok().and_then(|mut r| r.pop()).unwrap_or_default();
    }

    let today = chrono::Local::now().date_naive();
    let mut scored: Vec<JournalHit> = chunks
        .into_iter()
        .map(|c| {
            let kw = keyword_score(query, &c.text);
            let vec_score = if qvec.is_empty() {
                None
            } else {
                vectors.get(&c.id).map(|v| cosine(&qvec, v) as f64)
            };
            // With no vector signal the keyword score carries the full weight rather
            // than being scaled to 0.3 of itself — otherwise every result on a
            // machine without an embed model would fall under the floor.
            let blended = match vec_score {
                Some(v) => VECTOR_WEIGHT * v.max(0.0) + KEYWORD_WEIGHT * kw,
                None => kw,
            };
            let decay = if c.tier == Tier::Core { 1.0 } else { decay_factor(&c.date, today) };
            JournalHit { score: blended * decay, id: c.id, tier: c.tier, date: c.date, text: c.text }
        })
        .filter(|h| h.score >= JOURNAL_FLOOR)
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

/// Embed any chunks whose text changed since last time, and return the id→vector
/// map. The index is keyed by chunk id with a content hash, so an edited daily
/// section re-embeds and a consolidated-away core bullet drops out.
pub(super) async fn journal_vectors(
    chunks: &[crate::core::journal::Chunk],
    settings: &Settings,
    data_dir: &Path,
) -> Result<HashMap<String, Vec<f32>>, String> {
    let path = journal_index_path(data_dir);
    let mut idx: Index = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if idx.model != settings.embed_model || idx.format != INDEX_FORMAT {
        idx = Index { model: settings.embed_model.clone(), format: INDEX_FORMAT, entries: HashMap::new() };
    }

    let todo: Vec<(String, String, String)> = chunks
        .iter()
        .filter_map(|c| {
            let h = content_hash(&c.id, &c.text);
            let stale = idx.entries.get(&c.id).map(|e| e.hash != h).unwrap_or(true);
            stale.then(|| (c.id.clone(), h, c.text.chars().take(2000).collect()))
        })
        .collect();

    for batch in todo.chunks(16) {
        let vecs = embed(settings, batch.iter().map(|(_, _, t)| t.clone()).collect()).await?;
        for ((id, hash, _), vec) in batch.iter().zip(vecs) {
            idx.entries.insert(id.clone(), Entry { hash: hash.clone(), vec });
        }
    }

    let live: std::collections::HashSet<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
    idx.entries.retain(|k, _| live.contains(k.as_str()));
    if !todo.is_empty() {
        if let Ok(s) = serde_json::to_string(&idx) {
            let _ = crate::core::store::write_atomic(&path, &s);
        }
    }

    Ok(idx.entries.into_iter().map(|(k, e)| (k, e.vec)).collect())
}

/// Journal hits formatted for a prompt. Empty when nothing cleared the floor, so
/// the caller can append it unconditionally without spending tokens on a heading
/// over an empty list.
pub fn journal_block(hits: &[JournalHit]) -> String {
    use crate::core::journal::Tier;
    if hits.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n\n[FROM YOUR MEMORY JOURNAL — things established in past conversations]\n");
    for h in hits {
        match h.tier {
            Tier::Core => s.push_str(&format!("- {}\n", h.text.trim())),
            Tier::Daily => s.push_str(&format!("- ({}) {}\n", h.date, h.text.trim())),
        }
    }
    s
}
