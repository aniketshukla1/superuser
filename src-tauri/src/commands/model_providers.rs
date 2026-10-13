//! Model providers

use super::*;

/// The provider registry + which have a key + their curated models (for Settings).
#[tauri::command]
pub fn providers_list() -> Vec<crate::core::providers::ProviderInfo> {
    crate::core::providers::list_info()
}

/// Store an API key for a model provider (anthropic routes to the cloud key).
#[tauri::command]
pub fn set_provider_key(provider: String, key: String) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("empty key".into());
    }
    if provider == "anthropic" {
        secrets::set_cloud_key(key);
    } else if crate::core::providers::find(&provider).is_some() {
        secrets::set_provider_key(&provider, key);
    } else {
        return Err(format!("unknown provider: {provider}"));
    }
    Ok(())
}

/// Point a provider at a different OpenAI-compatible endpoint. Empty clears the
/// override and restores the vendor default.
#[tauri::command]
pub fn set_provider_base_url(provider: String, url: String) -> Result<(), String> {
    if crate::core::providers::find(&provider).is_none() {
        return Err(format!("unknown provider: {provider}"));
    }
    let url = crate::core::providers::validate_base_url(&url)?;
    if url.is_empty() {
        secrets::clear_provider_base_url(&provider);
    } else {
        secrets::set_provider_base_url(&provider, &url);
    }
    Ok(())
}

/// The model ids to offer for a provider whose list we can't curate. Empty clears
/// the override and restores the curated list.
#[tauri::command]
pub fn set_provider_models(provider: String, models: String) -> Result<(), String> {
    if crate::core::providers::find(&provider).is_none() {
        return Err(format!("unknown provider: {provider}"));
    }
    let models = models.trim();
    if models.is_empty() {
        secrets::clear_provider_models(&provider);
        return Ok(());
    }
    secrets::set_provider_models(&provider, models);
    Ok(())
}

/// The full connector catalog (40) + which are connected — for the connect UI.
#[tauri::command]
pub fn connectors_catalog() -> Vec<crate::integrations::descriptors::ConnectorInfo> {
    crate::integrations::descriptors::list_info()
}

/// Connect a REST-registry connector: verify the pasted credential against the
/// live API first, then store it (single-field connectors store the bare token;
/// multi-field ones store a JSON object). Returns the identity label.
#[tauri::command]
pub async fn connector_connect(id: String, fields: serde_json::Value) -> Result<String, String> {
    let desc = crate::integrations::descriptors::find(&id).ok_or_else(|| format!("unknown connector: {id}"))?;
    // Single secret field → store it bare; several → store the whole map.
    let stored = match (desc.fields.len(), fields.as_object()) {
        (1, Some(map)) => map.values().next().and_then(|v| v.as_str()).unwrap_or("").to_string(),
        (_, Some(_)) => fields.to_string(),
        _ => return Err("provide the connector's fields".into()),
    };
    if stored.trim().is_empty() {
        return Err("the credential is empty".into());
    }
    let identity = crate::integrations::verify_rest(&id, &stored).await?;
    secrets::set_integration_token(&id, &stored);
    secrets::set_account(&id, &identity);
    Ok(identity)
}
