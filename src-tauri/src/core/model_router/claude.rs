use super::*;

pub(super) async fn claude(settings: &Settings, messages: &[ChatMessage]) -> Result<(String, String), String> {
    let key = secrets::get_cloud_key().ok_or("No Anthropic API key in keychain.")?;

    // Anthropic wants the system prompt separate from the turn list.
    let system: String = messages
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let turns: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| {
            if m.images.is_empty() && m.docs.is_empty() {
                json!({ "role": m.role, "content": m.content })
            } else {
                // attachments become content blocks: text + images + PDFs
                let mut blocks = vec![json!({ "type": "text", "text": m.content })];
                for b64 in &m.images {
                    blocks.push(json!({
                        "type": "image",
                        "source": { "type": "base64", "media_type": sniff_media_type(b64), "data": b64 }
                    }));
                }
                for b64 in &m.docs {
                    blocks.push(json!({
                        "type": "document",
                        "source": { "type": "base64", "media_type": "application/pdf", "data": b64 }
                    }));
                }
                json!({ "role": m.role, "content": blocks })
            }
        })
        .collect();

    // Through the one client (`core::llm`), so this side call gets the same retry,
    // the same cancel handling and the same per-model thinking shape as the main loop
    // — the whole point of Phase 2. It had none of them: a 529 here failed outright.
    let req = crate::core::llm::LlmRequest::new(&settings.cloud_model, settings.max_tokens)
        .system(system)
        .messages(turns);
    // A side call has no stop flag of its own; it is short and nobody is watching it.
    let never = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let reply = crate::core::llm::call(&req, &key, &never)
        .await?
        .ok_or_else(|| "cancelled".to_string())?;
    Ok((reply.text(), format!("claude:{}", settings.cloud_model)))
}
