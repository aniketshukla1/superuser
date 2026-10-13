//! Plugins: Privacy Guard, Document Drafter, Smart Scheduler, Inbox Zero

use super::*;

/// Privacy Guard: redact PII locally (deterministic, no model/network).
#[tauri::command]
pub fn privacy_redact(text: String) -> crate::core::privacy::RedactOut {
    crate::core::privacy::redact(&text)
}

/// Pull up to `cap` memories whose text matches any hint (or whose category
/// matches `category`), formatted for a model prompt. Shared by the plugins.
pub(super) fn memory_pool(state: &AppState, category: &str, hints: &[&str], cap: usize) -> Vec<String> {
    let mut pool = Vec::new();
    for m in state.memory.list() {
        let hay = format!("{} {} {}", m.title, m.summary, m.body).to_lowercase();
        if m.category.eq_ignore_ascii_case(category) || hints.iter().any(|h| hay.contains(h)) {
            pool.push(format!("--- {} [{}] ({})\n{}", m.title, m.category, m.date, m.body.chars().take(600).collect::<String>()));
        }
        if pool.len() >= cap { break; }
    }
    pool
}

pub(super) fn plugin_settings(state: &AppState, model: &Option<String>) -> (Settings, bool) {
    let mut settings = state.settings.lock().unwrap().clone();
    let prefer = match model.as_deref().filter(|m| !m.is_empty()) {
        Some(m) if m.starts_with("claude") => { settings.cloud_model = m.to_string(); settings.cloud_enabled = true; settings.local_only = false; true }
        Some(m) => { settings.ollama_model = m.to_string(); false }
        None => false,
    };
    (settings, prefer)
}

/// Document Drafter: generate a document, grounded in the user's memories.
#[tauri::command]
pub async fn doc_draft(state: State<'_, AppState>, prompt: String, model: Option<String>) -> Result<String, String> {
    if prompt.trim().is_empty() { return Err("Describe the document you want.".into()); }
    let (settings, prefer) = plugin_settings(&state, &model);
    let cites = crate::core::embeddings::recall(&state.memory, &settings, &state.data_dir, &prompt, 8).await;
    let mut ctx = String::new();
    for c in &cites {
        if let Some(m) = state.memory.parse_file(&state.memory.vault_path.join(&c.filename)) {
            ctx.push_str(&format!("### {}\n{}\n\n", m.title, m.body.chars().take(900).collect::<String>()));
        }
    }
    let system = format!(
        "You are a document drafter. Write a polished, well-structured document in Markdown for the user's \
         request. Use clear headings, tight prose, and lists/tables where they help. Ground it in the \
         context from their memories below when relevant; don't invent facts about them. Output ONLY the \
         document.\n\nContext from their memories:\n{}",
        if ctx.is_empty() { "(none found)".into() } else { ctx }
    );
    let messages = vec![
        ChatMessage { role: "system".into(), content: system, images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: prompt, images: vec![], docs: vec![] },
    ];
    let (content, _) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer)).await?;
    Ok(content)
}

/// Smart Scheduler: upcoming meetings + free-slot suggestions from the calendar
/// memories. Returns JSON { events:[{time,title,detail}], gaps:[...], summary }.
#[tauri::command]
pub async fn schedule_view(state: State<'_, AppState>, model: Option<String>) -> Result<serde_json::Value, String> {
    let (settings, prefer) = plugin_settings(&state, &model);
    let pool = memory_pool(&state, "calendar", &["meeting", "call", "appointment", "invite", "standup", "sync", "1:1", "event", "booking"], 30);
    if pool.is_empty() {
        return Ok(serde_json::json!({ "events": [], "gaps": [], "summary": "", "empty": true }));
    }
    let now = chrono::Local::now();
    let system = format!(
        "Today is {}. From the user's calendar records below, list their upcoming meetings and suggest free \
         time. Return ONLY JSON: {{\"events\":[{{\"time\":\"09:30\",\"title\":\"...\",\"detail\":\"...\"}}],\
         \"gaps\":[{{\"time\":\"11:00–12:30\",\"title\":\"free\",\"detail\":\"good for focus work\"}}],\
         \"summary\":\"one sentence\"}}. Only real, upcoming items — never invent.",
        now.format("%A %e %B %H:%M")
    );
    let messages = vec![
        ChatMessage { role: "system".into(), content: system, images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: pool.join("\n\n").chars().take(16_000).collect::<String>(), images: vec![], docs: vec![] },
    ];
    let (raw, _) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer)).await?;
    let cleaned = clean_completion(&raw);
    let val: serde_json::Value = match (cleaned.find('{'), cleaned.rfind('}')) {
        (Some(a), Some(b)) if b > a => serde_json::from_str(&cleaned[a..=b]).unwrap_or_else(|_| serde_json::json!({})),
        _ => serde_json::json!({}),
    };
    Ok(serde_json::json!({
        "events": val.get("events").cloned().unwrap_or_else(|| serde_json::json!([])),
        "gaps": val.get("gaps").cloned().unwrap_or_else(|| serde_json::json!([])),
        "summary": val.get("summary").cloned().unwrap_or_else(|| serde_json::json!("")),
        "empty": false,
    }))
}

/// Inbox Zero: triage the user's email memories into buckets with a summary and
/// suggested action per item. Returns JSON { buckets:[{name, items:[...]}] }.
#[tauri::command]
pub async fn inbox_triage(state: State<'_, AppState>, model: Option<String>) -> Result<serde_json::Value, String> {
    let (settings, prefer) = plugin_settings(&state, &model);
    let pool = memory_pool(&state, "gmail", &["email", "re:", "fwd:", "unsubscribe", "inbox", "reply", "sent you"], 30);
    if pool.is_empty() {
        return Ok(serde_json::json!({ "buckets": [], "empty": true }));
    }
    let system =
        "You are an email triage assistant. Sort the user's emails below into buckets and, for each email, \
         give a one-line summary and a suggested next action. Return ONLY JSON: \
         {\"buckets\":[{\"name\":\"Action needed|Waiting on|FYI|Newsletters\",\"items\":[{\"from\":\"...\",\
         \"subject\":\"...\",\"summary\":\"...\",\"action\":\"...\"}]}]}. Keep buckets that have items; be concise; don't invent emails.";
    let messages = vec![
        ChatMessage { role: "system".into(), content: system.into(), images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: pool.join("\n\n").chars().take(16_000).collect::<String>(), images: vec![], docs: vec![] },
    ];
    let (raw, _) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer)).await?;
    let cleaned = clean_completion(&raw);
    let val: serde_json::Value = match (cleaned.find('{'), cleaned.rfind('}')) {
        (Some(a), Some(b)) if b > a => serde_json::from_str(&cleaned[a..=b]).unwrap_or_else(|_| serde_json::json!({ "buckets": [] })),
        _ => serde_json::json!({ "buckets": [] }),
    };
    Ok(serde_json::json!({ "buckets": val.get("buckets").cloned().unwrap_or_else(|| serde_json::json!([])), "empty": false }))
}

/// Inbox Zero: draft a reply to an email given its context + an instruction.
#[tauri::command]
pub async fn inbox_draft(state: State<'_, AppState>, subject: String, context: String, instruction: String, model: Option<String>) -> Result<String, String> {
    let (settings, prefer) = plugin_settings(&state, &model);
    let system = "You draft email replies in the user's voice: clear, warm and concise. Output ONLY the reply body (no subject line, no placeholders like [Name] unless truly unknown).";
    let user = format!("Email subject: {subject}\n\nContext / thread:\n{context}\n\nWrite a reply. Instruction: {}", if instruction.trim().is_empty() { "reply appropriately" } else { &instruction });
    let messages = vec![
        ChatMessage { role: "system".into(), content: system.into(), images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: user, images: vec![], docs: vec![] },
    ];
    let (content, _) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer)).await?;
    Ok(content)
}

/// Ollama health + installed models, so the UI can show status and offer a
/// real model picker.
#[tauri::command]
pub async fn ollama_status(state: State<'_, AppState>) -> Result<crate::core::ollama::OllamaStatus, String> {
    let settings = state.settings.lock().unwrap().clone();
    Ok(crate::core::ollama::status(&settings).await)
}
