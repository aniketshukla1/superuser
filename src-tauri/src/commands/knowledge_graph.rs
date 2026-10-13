use super::*;

#[derive(Serialize)]
pub struct ChatReply {
    pub content: String,
    pub citations: Vec<Citation>,
    pub provider: String,
    pub local_only: bool,
}

#[tauri::command]
pub fn list_memories(state: State<AppState>) -> Vec<Memory> {
    state.memory.list()
}

#[tauri::command]
pub fn save_memory(state: State<AppState>, mut input: MemoryInput) -> Result<String, String> {
    // This command *is* the user typing, so it is the one place allowed to claim
    // Internal. Set it here rather than trusting the payload: the field is
    // deserialized from the window, and a memory that says it is trusted because
    // the caller said so is not a trust boundary.
    input.origin = crate::core::memory::Origin::Internal;
    state.memory.save(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memory(state: State<AppState>, filename: String) -> bool {
    state.memory.delete(&filename)
}

/// Pull `[[Title]]` wikilink targets out of a memory body.
pub(super) fn wikilinks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = body.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'[' && b[i + 1] == b'[' {
            if let Some(end) = body[i + 2..].find("]]") {
                let inner = &body[i + 2..i + 2 + end];
                let name = inner.split('|').next().unwrap_or(inner).trim();
                if !name.is_empty() {
                    out.push(name.to_string());
                }
                i = i + 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Build the knowledge graph from real memories: You → categories → memories,
/// memories ↔ shared tags, and memory→memory via `[[wikilinks]]`.
#[tauri::command]
pub fn knowledge_graph(state: State<AppState>) -> serde_json::Value {
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    let mems = state.memory.list();
    let mut nodes = vec![json!({ "id": "you", "label": "You", "kind": "self" })];
    let mut edges: Vec<serde_json::Value> = Vec::new();

    // categories
    let mut seen_cat: HashSet<String> = HashSet::new();
    for m in &mems {
        let c = if m.category.trim().is_empty() { "Other".to_string() } else { m.category.clone() };
        if seen_cat.insert(c.clone()) {
            nodes.push(json!({ "id": format!("cat:{c}"), "label": c, "kind": "category" }));
            edges.push(json!({ "s": "you", "t": format!("cat:{c}") }));
        }
    }

    // tags shared by ≥2 memories become connector nodes (single-use tags add noise)
    let mut tag_counts: HashMap<String, u32> = HashMap::new();
    for m in &mems {
        for t in &m.tags {
            *tag_counts.entry(t.clone()).or_insert(0) += 1;
        }
    }
    let tag_nodes: HashSet<String> = tag_counts.iter().filter(|(_, c)| **c >= 2).map(|(t, _)| t.clone()).collect();
    for t in &tag_nodes {
        nodes.push(json!({ "id": format!("tag:{t}"), "label": format!("#{t}"), "kind": "tag" }));
    }

    // memories + their edges
    let title_map: HashMap<String, String> = mems.iter().map(|m| (m.title.to_lowercase(), m.id.clone())).collect();
    for m in &mems {
        let mid = format!("m:{}", m.id);
        nodes.push(json!({ "id": mid, "label": m.title, "kind": "memory", "weight": m.weight }));
        let c = if m.category.trim().is_empty() { "Other" } else { &m.category };
        edges.push(json!({ "s": format!("cat:{c}"), "t": mid }));
        for t in &m.tags {
            if tag_nodes.contains(t) {
                edges.push(json!({ "s": mid, "t": format!("tag:{t}") }));
            }
        }
        for link in wikilinks(&m.body) {
            if let Some(other) = title_map.get(&link.to_lowercase()) {
                if *other != m.id {
                    edges.push(json!({ "s": mid, "t": format!("m:{other}") }));
                }
            }
        }
    }

    // Curated knowledge pages, and the links the agent actually wrote between them.
    // The rest of this graph is inferred from categories and shared tags — these
    // edges are the only ones somebody meant.
    let kg = crate::core::knowledge::Knowledge::new(&state.memory.vault_path).graph();
    if let Some(kn) = kg["nodes"].as_array() {
        for n in kn {
            nodes.push(n.clone());
            if n["kind"] == "category" {
                edges.push(json!({ "s": "you", "t": n["id"] }));
            }
        }
    }
    if let Some(ke) = kg["edges"].as_array() {
        edges.extend(ke.iter().cloned());
    }

    json!({ "nodes": nodes, "edges": edges })
}

#[tauri::command]
pub async fn recall(state: State<'_, AppState>, query: String, limit: usize) -> Result<Vec<Citation>, String> {
    let settings = state.settings.lock().unwrap().clone();
    Ok(crate::core::embeddings::recall(&state.memory, &settings, &state.data_dir, &query, limit).await)
}

#[derive(serde::Deserialize)]
pub struct AttachmentIn {
    pub name: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub image_base64: Option<String>,
    #[serde(default)]
    pub pdf_base64: Option<String>,
}
