//! Messaging: outbound (send from the app to a platform)

use super::*;

/// Store the Telegram bot token in the keychain (empty string clears it).
#[tauri::command]
pub fn set_telegram_token(token: String) -> bool {
    if token.trim().is_empty() {
        secrets::clear_integration_token("telegram")
    } else {
        secrets::set_integration_token("telegram", token.trim())
    }
}

/// Whether Telegram is configured + the default chat id (token never returned).
#[tauri::command]
pub fn telegram_status(state: State<AppState>) -> serde_json::Value {
    serde_json::json!({
        "configured": secrets::has_integration_token("telegram"),
        "chat_id": state.settings.lock().unwrap().telegram_chat_id,
    })
}

/// Send a message to Telegram via the Bot API. Uses `chat_id` if given, else the
/// saved default. Returns Ok on a 200 from Telegram.
#[tauri::command]
pub async fn telegram_send(
    state: State<'_, AppState>,
    text: String,
    chat_id: Option<String>,
) -> Result<String, String> {
    let token = secrets::get_integration_token("telegram")
        .ok_or("No Telegram bot token — add one in Settings → Messaging.")?;
    let chat = chat_id
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| state.settings.lock().unwrap().telegram_chat_id.clone());
    if chat.trim().is_empty() {
        return Err("No Telegram chat id — set a default in Settings → Messaging, or include one.".into());
    }
    // chat_id may be a numeric id or an @channelusername.
    let chat_val: serde_json::Value = match chat.trim().parse::<i64>() {
        Ok(n) => serde_json::json!(n),
        Err(_) => serde_json::json!(chat.trim()),
    };
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let resp = crate::core::egress::client("telegram")
        .post(&url)
        .json(&serde_json::json!({ "chat_id": chat_val, "text": text }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Telegram: {e}"))?;
    if resp.status().is_success() {
        Ok("sent".into())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // Telegram returns a helpful "description" field on errors.
        let desc = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["description"].as_str().map(String::from))
            .unwrap_or(body);
        Err(format!("Telegram error {status}: {desc}"))
    }
}
