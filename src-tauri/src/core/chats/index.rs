use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages: Vec<StoredMessage>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatSummary {
    pub id: String,
    pub title: String,
    pub updated_at: i64,
    pub message_count: usize,
    pub preview: String,
    pub project_id: Option<String>,
}

pub(super) fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("chats.json")
}

pub(super) fn load_all(data_dir: &Path) -> Vec<Conversation> {
    std::fs::read_to_string(store_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(super) fn save_all(data_dir: &Path, all: &[Conversation]) {
    if let Ok(s) = serde_json::to_string(all) {
        let _ = crate::core::store::write_atomic(&store_path(data_dir), &s);
    }
}

pub(super) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(super) fn derive_title(messages: &[StoredMessage]) -> String {
    let first = messages.iter().find(|m| m.role == "user").map(|m| m.content.trim()).unwrap_or("New chat");
    let t: String = first.chars().take(60).collect();
    if t.is_empty() { "New chat".into() } else { t }
}

pub fn list(data_dir: &Path) -> Vec<ChatSummary> {
    let mut all = load_all(data_dir);
    all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    all.into_iter()
        .map(|c| {
            let preview = c.messages.last().map(|m| m.content.chars().take(90).collect::<String>()).unwrap_or_default();
            ChatSummary { id: c.id, title: c.title, updated_at: c.updated_at, message_count: c.messages.len(), preview, project_id: c.project_id }
        })
        .collect()
}

/// Move a conversation into a project (or out of one with None).
pub fn set_project(data_dir: &Path, chat_id: &str, project_id: Option<String>) {
    let mut all = load_all(data_dir);
    if let Some(c) = all.iter_mut().find(|c| c.id == chat_id) {
        c.project_id = project_id;
        save_all(data_dir, &all);
    }
}

pub fn get(data_dir: &Path, id: &str) -> Option<Conversation> {
    load_all(data_dir).into_iter().find(|c| c.id == id)
}

/// Upsert a conversation (fast — no LLM). Returns the id (generated if new).
pub fn save(data_dir: &Path, id: Option<String>, title: Option<String>, messages: Vec<StoredMessage>) -> String {
    let mut all = load_all(data_dir);
    let id = id.filter(|s| !s.is_empty()).unwrap_or_else(|| format!("chat-{}", now_ms()));
    let title = title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| derive_title(&messages));
    let now = now_ms();

    if let Some(existing) = all.iter_mut().find(|c| c.id == id) {
        existing.title = title;
        existing.messages = messages;
        existing.updated_at = now;
    } else {
        all.push(Conversation { id: id.clone(), title, created_at: now, updated_at: now, messages, project_id: None });
    }
    save_all(data_dir, &all);
    id
}

pub fn delete(data_dir: &Path, id: &str, memory: &MemoryEngine) {
    let mut all = load_all(data_dir);
    all.retain(|c| c.id != id);
    save_all(data_dir, &all);
    memory.delete(&format!("{id}.md"));
}

pub(super) fn transcript(conv: &Conversation) -> String {
    conv.messages
        .iter()
        .map(|m| format!("{}: {}", if m.role == "user" { "You" } else { "Brain" }, m.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Summarize a conversation with the local model. Falls back to a snippet if
/// the model is unavailable, so indexing never hard-fails.
pub(super) async fn summarize(settings: &Settings, conv: &Conversation) -> String {
    let body = transcript(conv);
    let msgs = vec![
        ChatMessage { role: "system".into(), content: "Summarize this conversation in 2-3 sentences, then list the key facts, decisions, or action items as short bullet points. Be concise and factual. Do not add a preamble.".into(), images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: body.chars().take(6000).collect(), images: vec![], docs: vec![] },
    ];
    match model_router::complete(settings, &msgs, model_router::Workload::Background).await {
        Ok((summary, _)) if !summary.trim().is_empty() => summary,
        _ => conv.messages.iter().find(|m| m.role == "user").map(|m| m.content.chars().take(160).collect()).unwrap_or_else(|| "A saved conversation.".into()),
    }
}

/// Write/refresh the searchable "Chat" memory for a conversation (summary +
/// transcript), so it surfaces in the Memory Tree and future RAG recall.
pub async fn index(data_dir: &Path, memory: &MemoryEngine, settings: &Settings, id: &str) -> Result<(), String> {
    let conv = get(data_dir, id).ok_or("Conversation not found")?;
    if conv.messages.is_empty() {
        return Ok(());
    }
    let summary = summarize(settings, &conv).await;
    let input = MemoryInput {
        filename: Some(format!("{id}.md")),
        title: format!("Chat: {}", conv.title),
        category: "Chat".into(),
        tags: vec!["chat".into(), "conversation".into()],
        summary: summary.lines().next().unwrap_or("Saved conversation").chars().take(160).collect(),
        body: format!("## Summary\n\n{summary}\n\n## Transcript\n\n{}", transcript(&conv)),
        // The user's own conversation with the assistant. They may have pasted
        // something from elsewhere into it, but they chose to — the chat is the
        // channel their instructions legitimately arrive on.
        origin: Origin::Internal,
        key: None,
        supersedes: None,
    };
    memory.save(&input).map_err(|e| e.to_string())?;
    Ok(())
}
