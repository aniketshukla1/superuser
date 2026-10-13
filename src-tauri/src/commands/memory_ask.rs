use super::*;

/// Answer a natural-language question about the user's memories (RAG + category
/// counts so "how many X" questions work). Powers the Memory Tree's Ask box.
#[tauri::command]
pub async fn memory_ask(state: State<'_, AppState>, query: String, model: Option<String>) -> Result<ChatReply, String> {
    use crate::core::model_router::{self, ChatMessage};
    if query.trim().is_empty() {
        return Err("Ask a question about your memories.".into());
    }
    let mut settings = state.settings.lock().unwrap().clone();
    let prefer_cloud = match model.as_deref().filter(|m| !m.is_empty()) {
        Some(m) if m.starts_with("claude") => { settings.cloud_model = m.to_string(); settings.cloud_enabled = true; settings.local_only = false; true }
        Some(m) => { settings.ollama_model = m.to_string(); false }
        None => false,
    };

    // Category counts over ALL memories, so counting/aggregation questions
    // ("how many git repos do I have?") can be answered accurately.
    let all = state.memory.list();
    let total = all.len();
    let mut cats: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for m in &all { *cats.entry(m.category.clone()).or_default() += 1; }
    let cat_lines = if cats.is_empty() {
        "(no memories yet)".to_string()
    } else {
        cats.iter().map(|(k, v)| format!("- {k}: {v}")).collect::<Vec<_>>().join("\n")
    };

    // Semantic recall (wider than chat's 4) for the specifics.
    let citations = crate::core::embeddings::recall(&state.memory, &settings, &state.data_dir, &query, 12).await;
    let mut ctx = String::new();
    for c in &citations {
        if let Some(m) = state.memory.parse_file(&state.memory.vault_path.join(&c.filename)) {
            ctx.push_str(&format!("### {} [{}]\n{}\n{}\n\n", m.title, m.category, m.summary, m.body.chars().take(700).collect::<String>()));
            let _ = state.memory.reinforce(&c.filename);
        }
    }

    let system = format!(
        "You are the user's private memory assistant. Answer their question using ONLY the memories below — their \
         own connected data (GitHub, Gmail, Calendar, Notion, notes, chats). Be concise and specific, citing concrete \
         details (names, dates, numbers). For counting / how-many questions, use the category counts. If the memories \
         don't contain the answer, say so plainly — never invent.\n\n\
         The user has {total} memories across categories:\n{cat_lines}\n\n\
         Relevant memories:\n{}",
        if ctx.is_empty() { "(no closely-related memories found)".to_string() } else { ctx }
    );
    let messages = vec![
        ChatMessage { role: "system".into(), content: system, images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: query, images: vec![], docs: vec![] },
    ];
    let (content, provider) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer_cloud)).await?;
    Ok(ChatReply { content, citations, provider, local_only: settings.local_only })
}
