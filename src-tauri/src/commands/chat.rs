use super::*;

#[tauri::command]
pub async fn chat(
    state: State<'_, AppState>,
    message: String,
    history: Vec<ChatMessage>,
    model: Option<String>,
    attachments: Option<Vec<AttachmentIn>>,
    web: Option<bool>,
    mode: Option<String>,
) -> Result<ChatReply, String> {
    use crate::core::ollama as ol;
    // Snapshot settings (don't hold the lock across the await).
    let mut settings = state.settings.lock().unwrap().clone();
    // A model picked in the composer overrides routing for this turn: a
    // "claude*" id goes to the cloud (needs a key), anything else to Ollama.
    let prefer_cloud = match model.as_deref().filter(|m| !m.is_empty()) {
        Some(m) if m.starts_with("claude") => { settings.cloud_model = m.to_string(); settings.cloud_enabled = true; settings.local_only = false; true }
        Some(m) => { settings.ollama_model = m.to_string(); false }
        None => false,
    };

    // Process attachments the local file reader already extracted: text goes into
    // the prompt; images go to the model as base64.
    let atts = attachments.unwrap_or_default();
    let images: Vec<String> = atts.iter().filter_map(|a| a.image_base64.clone()).collect();
    let docs: Vec<String> = atts.iter().filter_map(|a| a.pdf_base64.clone()).collect();
    let file_text: String = atts
        .iter()
        .filter_map(|a| a.text.as_ref().map(|t| format!("Attached file \"{}\":\n{t}", a.name)))
        .collect::<Vec<_>>()
        .join("\n\n");

    let will_use_cloud = prefer_cloud && !settings.local_only && settings.cloud_enabled && secrets::has_cloud_key();

    // Intelligent routing: an image needs a vision model. If we won't be using
    // Claude (which is multimodal), make sure a local vision model is selected —
    // auto-switching to an installed one, or guiding the user to get one.
    if !images.is_empty() && !will_use_cloud {
        // Asked of the capability Ollama reports, not of the model's name. The name
        // list has no entry that matches `gemma4:12b`, which reads images perfectly
        // well — so a user on it used to be switched to some other installed model
        // without being told, or told to pull one they did not need.
        let infos = ol::list_model_infos(&settings).await.unwrap_or_default();
        if !ol::supports_vision(&infos, &settings.ollama_model) {
            match infos.iter().find(|m| ol::supports_vision(&infos, &m.name)) {
                Some(v) => settings.ollama_model = v.name.clone(),
                None => return Err("To read an image you need a vision model. Install one with `ollama pull llama3.2-vision` (or `llava`), then pick it in the model menu — or switch to a Claude model.".into()),
            }
        }
    }
    // A scanned PDF (no extractable text) can only be read natively by Claude.
    if !docs.is_empty() && !will_use_cloud {
        return Err("This looks like a scanned PDF. Switch to a Claude model (enable cloud + add your key in Settings) to read it — local OCR isn't bundled.".into());
    }

    let web_on = web.unwrap_or(false);
    let mut context = String::new();
    // Grounding from the Memory Tree — but NOT in web mode, where personal chat
    // memories are just noise (and were leaking into the answer's sources).
    let citations = if web_on {
        Vec::new()
    } else {
        // The journal alongside the notes: core memory reaches every prompt anyway,
        // but a *daily* summary from three weeks ago only surfaces if something asks
        // for it. Decayed, so recent conversations outrank equally-similar old ones.
        let journal = crate::core::journal::Journal::new(&state.memory.vault_path);
        let hits = crate::core::embeddings::recall_journal(&journal, &settings, &state.data_dir, &message, 4).await;
        context.push_str(&crate::core::embeddings::journal_block(&hits));

        let cites = crate::core::embeddings::recall(&state.memory, &settings, &state.data_dir, &message, 4).await;
        if !cites.is_empty() {
            context.push_str("\n\n[RECALLED FROM YOUR PRIVATE MEMORY]\n");
            for c in &cites {
                if let Some(m) = state.memory.parse_file(&state.memory.vault_path.join(&c.filename)) {
                    context.push_str(&format!("File: {}\n{}\n\n", m.filename, m.body));
                }
                let _ = state.memory.reinforce(&c.filename);
            }
        }
        cites
    };

    // Web research agent: search the web, read the top pages, and fold them into
    // context with sources. The web toggle is an explicit per-turn opt-in to the
    // network, so it runs even in Local-only mode — and the answer is still
    // synthesized by the LOCAL model unless the user picked a cloud one, so page
    // content never reaches a cloud LLM in Local-only.
    if web_on {
        // Prefer a self-hosted SearXNG instance (cleaner, private) when configured.
        let search = if settings.searxng_url.is_empty() {
            crate::core::websearch::search(&message, 6).await
        } else {
            crate::core::websearch::search_searxng(&settings.searxng_url, &message, 6).await
        };
        match search {
            Ok(results) if !results.is_empty() => {
                context.push_str("\n\n[WEB SEARCH RESULTS — these are the only facts you may use. Each item is title — url, then a snippet, then (when available) page content.]\n");
                for (i, r) in results.iter().enumerate() {
                    context.push_str(&format!("\n[{}] {} — {}\nSnippet: {}\n", i + 1, r.title, r.url, r.snippet));
                }
                // Read the top pages for real content; skip ones that return little
                // (paywalled/JS-only sites yield boilerplate).
                for r in results.iter().take(4) {
                    if let Ok(text) = crate::core::websearch::fetch_text(&r.url, 4000).await {
                        if text.trim().len() > 200 {
                            context.push_str(&format!("\n--- Page content: {} ---\n{}\n", r.url, text));
                        }
                    }
                }
            }
            Ok(_) => context.push_str("\n\n[WEB SEARCH: no results found — tell the user you couldn't find anything.]\n"),
            Err(e) => context.push_str(&format!("\n\n[WEB SEARCH unavailable: {e} — tell the user.]\n")),
        }
    }

    // How to behave. Not frozen here: this path has no session of its own to freeze
    // against, and it is a one-shot completion rather than a cached conversation.
    let profile_block = crate::core::profile::Learned::load(&state.data_dir).system_block();
    // Core memory (Deep Dream's output). The profile is how the user wants to be
    // worked with; this is what past conversations established. Both are cheap and
    // deterministic, unlike recall, which only fires when the question matches.
    let core_memory = crate::core::journal::Journal::new(&state.memory.vault_path).system_block();
    let mode_directive = match mode.as_deref() {
        Some("plan") => " The user is in PLAN-FIRST mode: briefly outline your approach as short steps before carrying it out.",
        Some("ask") => " The user is in ASK-FIRST mode: if fulfilling this would need an external action (web, syncing an integration, or acting in another app), ask them to confirm before doing it.",
        _ => "",
    };
    let web_directive = if web_on {
        " WEB SEARCH IS ON: live results (with page excerpts) are included below under [WEB SEARCH RESULTS]. \
         Base your answer ONLY on those results — be specific and concrete (real names, dates, numbers, headlines), \
         cite each claim inline as a markdown link to its source URL, and finish with a \"Sources:\" list of the links you used. \
         If the results don't actually contain the answer, say so plainly. Do NOT pad with generic background knowledge or templated lists."
    } else {
        ""
    };
    let system = format!(
        "You are Superuser, the user's private, local-first personal superintelligence. \
         Be concise and helpful. Use the recalled memory below when relevant, but never \
         mention the mechanics of recall.{}{}{}{}{}",
        mode_directive, web_directive, profile_block, core_memory, context
    );

    let user_content = if file_text.is_empty() { message.clone() } else { format!("{file_text}\n\n{message}") };
    let mut messages = vec![ChatMessage { role: "system".into(), content: system, images: vec![], docs: vec![] }];
    messages.extend(history);
    messages.push(ChatMessage { role: "user".into(), content: user_content, images, docs });

    let (content, provider) = model_router::complete(&settings, &messages, model_router::Workload::Foreground(prefer_cloud)).await?;

    Ok(ChatReply {
        content,
        citations,
        provider,
        local_only: settings.local_only,
    })
}
