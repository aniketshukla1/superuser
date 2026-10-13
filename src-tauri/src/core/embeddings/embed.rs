use super::*;

/// Bumped whenever the *shape* of what is indexed changes, as opposed to the model
/// that indexed it. Going from one vector per file to one per chunk re-keys every
/// entry: without this the old entries would sit there unmatched and un-evicted,
/// and `retain` would quietly drop them one stale read at a time.
pub(super) const INDEX_FORMAT: u32 = 2;

#[derive(Serialize, Deserialize, Default)]
pub(super) struct Index {
    pub(super) model: String,
    /// Absent in an index written before chunking, which is exactly the case that
    /// has to be thrown away.
    #[serde(default)]
    pub(super) format: u32,
    pub(super) entries: HashMap<String, Entry>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Entry {
    pub(super) hash: String,
    pub(super) vec: Vec<f32>,
}

pub(super) fn index_path(data_dir: &Path) -> PathBuf {
    data_dir.join("embeddings.json")
}

pub(super) fn load_index(data_dir: &Path) -> Index {
    std::fs::read_to_string(index_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(super) fn save_index(data_dir: &Path, idx: &Index) {
    if let Ok(s) = serde_json::to_string(idx) {
        let _ = crate::core::store::write_atomic(&index_path(data_dir), &s);
    }
}

pub(super) fn content_hash(title: &str, body: &str) -> String {
    let mut h = Sha256::new();
    h.update(title.as_bytes());
    h.update(body.as_bytes());
    format!("{:x}", h.finalize())[..16].to_string()
}

/// Relevance of each tool to what the user just asked, name -> cosine score.
///
/// HARNESS-RESEARCH-2026 §4's missing piece. Every 2026 paper that measured a
/// smaller catalogue helping selected **by relevance to the current query**; we
/// selected by a fixed priority order and measured the opposite. This is the
/// function that lets the two be compared on our own board.
///
/// Embeds `name: description` for each tool and the query in one batch — one round
/// trip, not one per tool. An embedder that is unavailable returns an empty map
/// rather than an error, because a missing ranking must degrade to "send the
/// always-sent set and fill" and never to a turn with no tools at all.
pub async fn tool_relevance(
    settings: &Settings,
    specs: &[crate::core::agent::ToolSpec],
    query: &str,
) -> std::collections::BTreeMap<String, f32> {
    let mut out = std::collections::BTreeMap::new();
    if specs.is_empty() || query.trim().is_empty() {
        return out;
    }
    let mut inputs: Vec<String> = specs
        .iter()
        .map(|t| format!("{}: {}", t.name, t.description))
        .collect();
    inputs.push(query.to_string());
    let Ok(vecs) = embed(settings, inputs).await else {
        return out;
    };
    let Some(q) = vecs.last() else { return out };
    for (t, v) in specs.iter().zip(vecs.iter()) {
        out.insert(t.name.clone(), cosine(v, q));
    }
    out
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na.sqrt() * nb.sqrt()) }
}

/// Batch-embed texts via Ollama's /api/embed.
pub async fn embed(settings: &Settings, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
    if inputs.is_empty() {
        return Ok(vec![]);
    }
    let url = format!("{}/api/embed", settings.ollama_url.trim_end_matches('/'));
    let client = crate::core::egress::client("embeddings");
    let resp = client
        .post(&url)
        // Held resident like the chat model. An embedding model is small (nomic-embed
        // is a few hundred MB against the chat model's several GB), and reloading one
        // per batch is both slow in itself and a second eviction pressure on the model
        // the user is actually waiting for.
        .json(&json!({
            "model": settings.embed_model,
            "input": inputs,
            "keep_alive": crate::core::ollama::KEEP_ALIVE,
        }))
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Ollama unreachable: {e}"))?;
    if !resp.status().is_success() {
        // 501 is the specific, common misconfiguration and it is not what the old
        // message said. Ollama answers it when the model is present and simply is
        // not an embedding model — a chat model in the `embed_model` box. "Is it
        // pulled?" sends the user to check the one thing that was never wrong.
        if resp.status().as_u16() == 501 {
            return Err(format!(
                "'{}' can't produce embeddings — it's a chat model, not an embedding model. \
                 Pick one in Settings → Models → Memory search (e.g. `nomic-embed-text`, \
                 `ollama pull nomic-embed-text` if it isn't installed).",
                settings.embed_model
            ));
        }
        return Err(format!("Ollama embed returned {} — is '{}' pulled?", resp.status(), settings.embed_model));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let arr = v["embeddings"].as_array().ok_or("No embeddings in Ollama response")?;
    Ok(arr
        .iter()
        .map(|e| e.as_array().map(|xs| xs.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()).unwrap_or_default())
        .collect())
}

/// Semantic recall with automatic fallback to keyword matching.
pub async fn recall(
    engine: &MemoryEngine,
    settings: &Settings,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> Vec<Citation> {
    recall_explained(engine, settings, data_dir, query, limit).await.0
}

/// The same recall, and *why* it fell back when it did.
///
/// The fallback is the right behaviour — a memory you can't search semantically is
/// better searched crudely than not at all — but it was completely silent, and a
/// silent permanent downgrade is indistinguishable from the feature working.
///
/// Found the hard way on the author's own machine: `embed_model` was set to a chat
/// model, Ollama answered every embed request with `501 This server does not support
/// embeddings`, and semantic recall had therefore never once run. The app looked
/// fine. Recall was keyword matching, the on-disk vector index sat frozen for
/// months at a different model, and nothing anywhere said so.
///
/// `None` means the vectors really were used.
pub async fn recall_explained(
    engine: &MemoryEngine,
    settings: &Settings,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> (Vec<Citation>, Option<String>) {
    if query.trim().is_empty() {
        return (vec![], None);
    }
    let (hits, why) = recall_memories(engine, settings, data_dir, query, limit).await;
    let cites = hits
        .into_iter()
        .map(|(m, s)| Citation {
            filename: m.filename,
            title: m.title,
            category: m.category,
            score: (s * 100.0).round() as i64,
        })
        .collect();
    (cites, why)
}
