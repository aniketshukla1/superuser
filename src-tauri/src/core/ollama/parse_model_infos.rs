use super::*;

/// Parse `/api/tags` into full [`ModelInfo`]s (name + size + tool support).
pub fn parse_model_infos(tags: &Value) -> Vec<ModelInfo> {
    tags["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    let name = m["name"].as_str()?.to_string();
                    let has = |cap: &str| {
                        m["capabilities"]
                            .as_array()
                            .map(|c| c.iter().any(|v| v.as_str() == Some(cap)))
                            .unwrap_or(false)
                    };
                    Some(ModelInfo {
                        name,
                        size: m["size"].as_u64().unwrap_or(u64::MAX),
                        tools: has("tools"),
                        thinking: has("thinking"),
                        vision: has("vision"),
                        reported_capabilities: m["capabilities"].is_array(),
                        // Ollama ≥0.30 reports this in the tag listing, so the real
                        // window costs no extra round trip. `/api/show` carries the
                        // same number under `<arch>.context_length`; this one is
                        // architecture-independent and already in hand.
                        context_length: m["details"]["context_length"]
                            .as_u64()
                            .and_then(|v| u32::try_from(v).ok()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Fetch installed models from Ollama. Empty + running=false if unreachable.
pub async fn list_models(settings: &Settings) -> Result<Vec<String>, String> {
    Ok(fetch_tags(settings).await.map(|v| parse_models(&v))?)
}

/// Same fetch, keeping the size and capability metadata that picking a model needs.
pub async fn list_model_infos(settings: &Settings) -> Result<Vec<ModelInfo>, String> {
    Ok(fetch_tags(settings).await.map(|v| parse_model_infos(&v))?)
}

pub(super) async fn fetch_tags(settings: &Settings) -> Result<Value, String> {
    let url = format!("{}/api/tags", settings.ollama_url.trim_end_matches('/'));
    let client = crate::core::egress::client("model");
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await
        .map_err(|e| format!("Could not reach Ollama at {}: {e}", settings.ollama_url))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn status(settings: &Settings) -> OllamaStatus {
    match list_models(settings).await {
        Ok(models) => OllamaStatus {
            chat_model_ok: model_available(&models, &settings.ollama_model),
            // Installed is not the same as able to embed, and reporting the first
            // as the second is how a chat model sat in the embedding slot unnoticed:
            // Ollama answered every request `501 This server does not support
            // embeddings`, semantic recall silently fell back to keyword matching for
            // months, and this flag said the model was fine the whole time. The name
            // test already exists — it is used to keep embedding models OUT of the
            // chat picker, and this is the same question asked the other way round.
            embed_model_ok: model_available(&models, &settings.embed_model)
                && is_embed_model(&settings.embed_model),
            chat_model: settings.ollama_model.clone(),
            embed_model: settings.embed_model.clone(),
            models,
            running: true,
            error: None,
        },
        Err(e) => OllamaStatus {
            running: false,
            models: vec![],
            chat_model: settings.ollama_model.clone(),
            chat_model_ok: false,
            embed_model: settings.embed_model.clone(),
            embed_model_ok: false,
            error: Some(e),
        },
    }
}
