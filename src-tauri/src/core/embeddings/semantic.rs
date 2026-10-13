use super::*;

/// How much of a memory goes into one vector.
///
/// What used to sit here was `take(2000)` over the *whole memory*: a note longer
/// than that was embedded on its opening and everything after it was unretrievable
/// by vector, permanently, with nothing anywhere saying so. The same cap is still on
/// the journal path below and is harmless there — the journal's unit is one
/// `## HH:MM` block, so the guard almost never fires — which is the point. The vault's
/// unit was a whole file, and the cap was load-bearing on every long note.
///
/// The fix is the journal's unit, not a bigger number: split the memory and index the
/// pieces.
pub(super) const CHUNK_CHARS: usize = 1200;

/// Weight on semantic similarity for the vault, matching the journal's blend.
///
/// The vault had no blend at all — `recall_explained` ran the vectors and fell back
/// to keywords when they failed or found nothing, which is two retrievers taking
/// turns. The exact tokens embeddings smear over (a repo name, an error code, a
/// person's name) are most of what is *in* a synced vault, so the keyword half is
/// not a fallback here, it is half the signal.
pub(super) const VAULT_VECTOR_WEIGHT: f64 = 0.7;

/// Below this a memory is noise rather than a weak match. Was an inline `0.30`
/// against a bare cosine; named here beside the journal's floor because it is the
/// knob that decides how often recall comes back empty, and it now applies to a
/// blended, weight-scaled score rather than to a raw similarity.
pub(super) const VAULT_FLOOR: f64 = 0.22;

/// One memory, split into indexable pieces.
///
/// Splits on markdown headings, then on blank lines once a piece is over the cap, so
/// a heading-less wall of text still gets divided somewhere sensible. Every piece
/// carries the memory's title, which costs a few tokens and buys a chunk that is
/// self-describing: "the retry limit is 5" is not retrievable on its own, and
/// "Deploy pipeline notes / the retry limit is 5" is.
pub(super) fn chunks_of(m: &crate::core::memory::Memory) -> Vec<(String, String)> {
    let mut sections: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in m.body.lines() {
        let heading = line.starts_with('#') && !cur.trim().is_empty();
        let overlong = cur.len() >= CHUNK_CHARS && line.trim().is_empty();
        if heading || overlong {
            sections.push(std::mem::take(&mut cur));
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        sections.push(cur);
    }
    // A memory with an empty body is still worth indexing on its title and summary.
    if sections.is_empty() {
        sections.push(String::new());
    }
    sections
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            let text = format!("{}\n{}\n{}", m.title, m.summary, s.trim());
            (format!("{}#{i}", m.filename), text.chars().take(CHUNK_CHARS * 2).collect())
        })
        .collect()
}

pub(super) async fn semantic(
    engine: &MemoryEngine,
    settings: &Settings,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<(crate::core::memory::Memory, f64)>, String> {
    use crate::core::journal::keyword_score;

    // `recallable`, not `list`: superseded and suspect memories are excluded in one
    // place so the vector path and the keyword path cannot disagree about what the
    // vault contains.
    let memories = engine.recallable();
    if memories.is_empty() {
        return Ok(vec![]);
    }

    let mut idx = load_index(data_dir);
    if idx.model != settings.embed_model || idx.format != INDEX_FORMAT {
        // Model changed, or the index is keyed the old way. Either way it is scrap.
        idx = Index { model: settings.embed_model.clone(), format: INDEX_FORMAT, entries: HashMap::new() };
    }

    // (chunk id, owning memory, text)
    let chunks: Vec<(String, &crate::core::memory::Memory, String)> =
        memories.iter().flat_map(|m| chunks_of(m).into_iter().map(move |(id, text)| (id, m, text))).collect();

    // Find chunks that need (re-)embedding.
    let mut todo: Vec<(String, String, String)> = vec![]; // (id, hash, text)
    for (id, m, text) in &chunks {
        let h = content_hash(&m.title, text);
        let stale = idx.entries.get(id).map(|e| e.hash != h).unwrap_or(true);
        if stale {
            todo.push((id.clone(), h, text.clone()));
        }
    }

    // Embed in batches of 16 to keep requests snappy.
    for batch in todo.chunks(16) {
        let vecs = embed(settings, batch.iter().map(|(_, _, t)| t.clone()).collect()).await?;
        for ((id, hash, _), vec) in batch.iter().zip(vecs) {
            idx.entries.insert(id.clone(), Entry { hash: hash.clone(), vec });
        }
    }
    // Drop chunks whose memory was deleted, or that a shorter edit removed.
    let live: std::collections::HashSet<&str> = chunks.iter().map(|(id, _, _)| id.as_str()).collect();
    idx.entries.retain(|k, _| live.contains(k.as_str()));
    if !todo.is_empty() {
        save_index(data_dir, &idx);
    }

    let qvec = embed(settings, vec![query.to_string()]).await?.into_iter().next().unwrap_or_default();
    if qvec.is_empty() {
        return Err("empty query embedding".into());
    }

    // Score every chunk, then keep each memory's best one. A memory is as relevant
    // as its most relevant part — averaging across chunks is what the old
    // one-vector-per-file index was doing implicitly, and it is why a long note with
    // one sharp paragraph in it ranked below a short vague one.
    let mut best: std::collections::HashMap<&str, (f64, &crate::core::memory::Memory)> = std::collections::HashMap::new();
    for (id, m, text) in &chunks {
        let Some(e) = idx.entries.get(id) else { continue };
        let cos = cosine(&qvec, &e.vec) as f64;
        // The weight the vault has always kept and the vector path has never read:
        // decayed by age, bumped by use. Only the keyword path applied it, so
        // reinforcing a memory made it rank higher exactly on the machines where
        // semantic recall was broken.
        // The same trust multiplier the keyword path applies, for the same reason:
        // between two memories that fit equally, the one the user wrote is the one
        // they meant. Provenance is a relevance signal, not only a safety label.
        let trust = if m.origin == crate::core::memory::Origin::Internal { 1.15 } else { 1.0 };
        let score =
            (VAULT_VECTOR_WEIGHT * cos.max(0.0) + (1.0 - VAULT_VECTOR_WEIGHT) * keyword_score(query, text)) * m.weight * trust;
        let slot = best.entry(m.filename.as_str()).or_insert((f64::MIN, m));
        if score > slot.0 {
            *slot = (score, m);
        }
    }

    let mut scored: Vec<(f64, &crate::core::memory::Memory)> = best.into_values().filter(|(s, _)| *s >= VAULT_FLOOR).collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    Ok(scored.into_iter().map(|(s, m)| (m.clone(), s)).collect())
}

/// Recall that keeps the bodies, and says whether the vectors were used.
///
/// The shared core of every vault recall. `recall_explained` projects it to
/// citations for the UI; the agent's `recall_memory` needs the text itself, and
/// until this existed it used the keyword-only `recall_scored` instead — so the
/// loop the scoreboard grades was the one path in the app that never touched
/// semantic recall at all.
pub async fn recall_memories(
    engine: &MemoryEngine,
    settings: &Settings,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> (Vec<(crate::core::memory::Memory, f64)>, Option<String>) {
    if query.trim().is_empty() {
        return (vec![], None);
    }
    match semantic(engine, settings, data_dir, query, limit).await {
        Ok(hits) if !hits.is_empty() => (hits, None),
        // Nothing cleared the floor. The vectors worked; the vault simply has
        // nothing close, so keyword gets a turn without anything being wrong.
        Ok(_) => (engine.recall_scored(query, limit), None),
        Err(e) => (engine.recall_scored(query, limit), Some(e)),
    }
}
