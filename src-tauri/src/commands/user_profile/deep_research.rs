use super::super::*;

#[tauri::command]
pub fn get_profile(state: State<AppState>) -> crate::core::profile::Profile {
    crate::core::profile::load(&state.data_dir)
}

/// Save the profile and mirror it into the vault as a durable "User Profile"
/// memory (so it's visible in the Memory Tree and recallable).
#[tauri::command]
pub fn save_profile(state: State<AppState>, profile: crate::core::profile::Profile) {
    crate::core::profile::save(&state.data_dir, &profile);
    // The form's fields are re-derived as observations on every load, so saving it
    // is enough to change what the profile says — but sessions already open are
    // holding a frozen copy, and someone who has just edited their own name should
    // not have to start a new conversation to be called by it.
    crate::core::profile::thaw_all();
    let learned = crate::core::profile::Learned::load(&state.data_dir);
    let _ = learned.write_profile_md(&state.memory.vault_path);
    let engine = crate::core::memory::MemoryEngine::new(state.memory.vault_path.clone());
    if profile.is_empty() {
        engine.delete("user-profile.md");
        return;
    }
    let name = if !profile.call_me.trim().is_empty() { profile.call_me.trim() } else { profile.name.trim() };
    let _ = engine.save(&MemoryInput {
        filename: Some("user-profile.md".into()),
        title: if name.is_empty() { "User Profile".into() } else { format!("User Profile — {name}") },
        category: "Profile".into(),
        tags: vec!["profile".into(), "user".into()],
        summary: "Who the user is and how they like the assistant to work.".into(),
        body: crate::core::profile::memory_body(&profile),
        origin: crate::core::memory::Origin::Internal, // the user described themselves
        key: None,
        supersedes: None,
    });
}

/// Deep research agent: iterative search → read → reflect → new query (a few
/// cycles), then write a cited report. Streams progress via "deep-research".
#[tauri::command]
pub async fn deep_research(app: tauri::AppHandle, state: State<'_, AppState>, topic: String, model: Option<String>, mode: Option<String>) -> Result<ChatReply, String> {
    use crate::core::{model_router::{self, ChatMessage}, websearch};
    use tauri::Emitter;

    const MAX_CYCLES: usize = 3;
    let mut settings = state.settings.lock().unwrap().clone();
    let prefer_cloud = match model.as_deref().filter(|m| !m.is_empty()) {
        Some(m) if m.starts_with("claude") => { settings.cloud_model = m.to_string(); settings.cloud_enabled = true; settings.local_only = false; true }
        Some(m) => { settings.ollama_model = m.to_string(); false }
        None => false,
    };
    let msg = |role: &str, content: String| ChatMessage { role: role.into(), content, images: vec![], docs: vec![] };

    // Plan-first mode: return a research plan to approve, without searching yet.
    if mode.as_deref() == Some("plan") {
        let _ = app.emit("deep-research", serde_json::json!({ "phase": "writing", "detail": "Drafting a research plan…" }));
        let sys = "You are planning research. For the user's topic, produce a short research PLAN as a numbered list of the key sub-questions you'd investigate (4-7 items). No preamble, no answers — just the plan. End with: \"Turn off Plan-first mode to run this.\"";
        let (content, provider) = model_router::complete(&settings, &[msg("system", sys.into()), msg("user", format!("Topic: {topic}"))], model_router::Workload::Foreground(prefer_cloud)).await?;
        let _ = app.emit("deep-research", serde_json::json!({ "phase": "done", "detail": "Plan ready" }));
        return Ok(ChatReply { content: format!("**Research plan**\n\n{content}"), citations: vec![], provider, local_only: settings.local_only });
    }

    let mut notes = String::new();
    let mut query = topic.trim().to_string();

    for cycle in 0..MAX_CYCLES {
        let _ = app.emit("deep-research", serde_json::json!({ "phase": "searching", "detail": format!("Searching: {query}"), "cycle": cycle + 1, "total": MAX_CYCLES }));
        let results = if settings.searxng_url.is_empty() {
            websearch::search(&query, 6).await
        } else {
            websearch::search_searxng(&settings.searxng_url, &query, 6).await
        }
        .unwrap_or_default();

        if results.is_empty() {
            notes.push_str(&format!("\n## {query}\n(no results found)\n"));
        } else {
            notes.push_str(&format!("\n## Findings for: {query}\n"));
            for r in results.iter().take(5) {
                notes.push_str(&format!("- {} — {}\n  {}\n", r.title, r.url, r.snippet));
            }
            let _ = app.emit("deep-research", serde_json::json!({ "phase": "reading", "detail": "Reading top pages…", "cycle": cycle + 1, "total": MAX_CYCLES }));
            for r in results.iter().take(2) {
                if let Ok(text) = websearch::fetch_text(&r.url, 3000).await {
                    if text.trim().len() > 200 {
                        notes.push_str(&format!("\n### Page: {}\n{}\n", r.url, text));
                    }
                }
            }
        }

        // Reflect on gaps and pick the next query (skip after the final cycle).
        if cycle + 1 < MAX_CYCLES {
            let _ = app.emit("deep-research", serde_json::json!({ "phase": "reflecting", "detail": "Identifying gaps…", "cycle": cycle + 1, "total": MAX_CYCLES }));
            let sys = "You are planning research. Given the topic and the notes gathered so far, output ONE concise web search query (plain text, no quotes, no preamble) that fills the most important remaining gap. If the notes already cover the topic well, output exactly: DONE";
            let user = format!("Topic: {topic}\n\nNotes so far:\n{}", notes.chars().take(6000).collect::<String>());
            match model_router::complete(&settings, &[msg("system", sys.into()), msg("user", user)], model_router::Workload::Foreground(prefer_cloud)).await {
                Ok((next, _)) => {
                    let next = next.trim().lines().next().unwrap_or("").trim().to_string();
                    if next.is_empty() || next.to_uppercase().contains("DONE") { break; }
                    query = next;
                }
                Err(_) => break,
            }
        }
    }

    let _ = app.emit("deep-research", serde_json::json!({ "phase": "writing", "detail": "Writing the report…" }));
    let sys = "You are a research analyst. Write a clear, well-structured report answering the user's topic using ONLY the research notes provided. Use markdown headings and bullet points, be specific and concrete, and cite claims inline as markdown links to their source URLs. End with a \"Sources\" section listing the links you used. Do not invent facts that aren't in the notes.";
    let user = format!("Topic: {topic}\n\nResearch notes:\n{}", notes.chars().take(24000).collect::<String>());
    let (content, provider) = model_router::complete(&settings, &[msg("system", sys.into()), msg("user", user)], model_router::Workload::Foreground(prefer_cloud)).await?;
    let _ = app.emit("deep-research", serde_json::json!({ "phase": "done", "detail": "Done" }));

    Ok(ChatReply { content, citations: vec![], provider, local_only: settings.local_only })
}

/// Pull an Ollama model (e.g. a vision model so images can be read locally),
/// streaming progress to the UI via the "model-pull" event.
#[tauri::command]
pub async fn pull_model(app: tauri::AppHandle, state: State<'_, AppState>, model: String) -> Result<(), String> {
    use tauri::Emitter;
    let ollama_url = state.settings.lock().unwrap().ollama_url.clone();
    let url = format!("{}/api/pull", ollama_url.trim_end_matches('/'));
    let client = crate::core::egress::client("model");
    let mut resp = client
        .post(&url)
        .json(&serde_json::json!({ "model": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Ollama to download the model. Is it running? ({e})"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama couldn't pull “{model}” ({}).", resp.status()));
    }

    let mut buf = String::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(nl) = buf.find('\n') {
            let line: String = buf.drain(..=nl).collect();
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(err) = v["error"].as_str() {
                    return Err(err.to_string());
                }
                let status = v["status"].as_str().unwrap_or("").to_string();
                let total = v["total"].as_f64().unwrap_or(0.0);
                let completed = v["completed"].as_f64().unwrap_or(0.0);
                let percent = if total > 0.0 { (completed / total * 100.0).round() } else { 0.0 };
                let _ = app.emit("model-pull", serde_json::json!({ "model": model, "status": status, "percent": percent }));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    let mut s = state.settings.lock().unwrap().clone();
    s.has_cloud_key = secrets::has_cloud_key();
    s
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, patch: serde_json::Value) -> Settings {
    let mut guard = state.settings.lock().unwrap();
    guard.apply_patch(&patch);
    guard.save(&state.data_dir);
    let mut out = guard.clone();
    out.has_cloud_key = secrets::has_cloud_key();
    out
}

#[tauri::command]
pub fn set_cloud_key(key: String) -> bool {
    secrets::set_cloud_key(&key)
}

#[tauri::command]
pub fn list_integrations(state: State<AppState>) -> Vec<crate::integrations::Integration> {
    state.registry.list(&state.memory)
}

/// Validate a token, store it only if it works, and return the account label.
#[tauri::command]
pub async fn connect_token_integration(id: String, token: String) -> Result<String, String> {
    crate::integrations::connect_token(&id, &token).await
}
