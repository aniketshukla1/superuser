use super::*;

/// The channels `send` can actually deliver to.
///
/// This is the authority for "can a message leave through this source", and it sits
/// beside the match it mirrors so the two cannot drift — a test below walks it and
/// fails if `send` stops routing one. The Access panel offers a Read/Write choice for
/// exactly these: offering it for a source nothing can be sent through would be a
/// control that means nothing, and withholding it from one that CAN send would leave
/// a way out of the machine the user cannot close.
pub const SENDABLE: &[&str] = &["telegram", "slack", "discord", "whatsapp", "email"];

/// Whether a message can leave through `channel` (a connector id, case-insensitive).
pub fn is_sendable(channel: &str) -> bool {
    SENDABLE.iter().any(|c| c.eq_ignore_ascii_case(channel))
}

/// Split `channel:address`; defaults to Telegram (the wired outbound channel).
pub fn parse_target(target: &str) -> (String, String) {
    match target.split_once(':') {
        Some((ch, addr)) if !ch.is_empty() && !addr.is_empty() => (ch.to_lowercase(), addr.to_string()),
        _ => ("telegram".into(), target.to_string()),
    }
}

/// Deliver a message. Returns a short confirmation for the agent's transcript.
pub async fn send(target: &str, text: &str) -> Result<String, String> {
    send_threaded(target, text, None, None).await
}

/// `send`, for a message that answers another. An email names the one it answers
/// (`In-Reply-To`, `References`), so the reply lands in the sender's thread; the other
/// channels have no thread to name and ignore both.
pub async fn send_threaded(target: &str, text: &str, in_reply_to: Option<&str>, references: Option<&str>) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("nothing to send (empty message)".into());
    }
    let (channel, address) = parse_target(target);
    match channel.as_str() {
        "telegram" => send_telegram(&address, text).await,
        "slack" => send_slack(&address, text).await,
        "discord" => send_discord(&address, text).await,
        "whatsapp" => send_whatsapp(&address, text).await,
        "email" | "mailto" => {
            // SMTP is blocking; keep it off the async runtime's worker threads.
            // The first line becomes the subject when it's followed by a blank line.
            let (subject, body) = match text.split_once("\n\n") {
                Some((s, b)) if s.lines().count() == 1 && s.len() <= 120 => (s.to_string(), b.to_string()),
                _ => ("Message from your Superuser assistant".to_string(), text.to_string()),
            };
            let to = address.clone();
            let (in_reply_to, references) = (in_reply_to.map(str::to_string), references.map(str::to_string));
            tokio::task::spawn_blocking(move || {
                crate::integrations::email::send_reply(&to, &subject, &body, in_reply_to.as_deref(), references.as_deref())
            })
                .await
                .map_err(|e| format!("email task failed: {e}"))?
        }
        other => Err(format!("'{other}' isn't a sendable channel yet — available: telegram, slack, discord, whatsapp, email.")),
    }
}

pub(super) async fn send_telegram(chat_id: &str, text: &str) -> Result<String, String> {
    let token = secrets::get_integration_token("telegram")
        .ok_or("Telegram isn't connected — add its bot token in Integrations.")?;
    if chat_id.trim().is_empty() {
        return Err("no Telegram chat id — set a default in Settings → Messaging.".into());
    }
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let resp = crate::core::egress::client("slack")
        .post(&url)
        .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
        .send()
        .await
        .map_err(|e| format!("Telegram request failed: {e}"))?;
    if resp.status().is_success() {
        Ok(format!("Sent to Telegram ({chat_id})."))
    } else {
        let t = resp.text().await.unwrap_or_default();
        Err(format!("Telegram rejected the message: {}", t.chars().take(160).collect::<String>()))
    }
}

pub(super) async fn send_slack(channel: &str, text: &str) -> Result<String, String> {
    let token = secrets::get_integration_token("slack")
        .ok_or("Slack isn't connected — add a bot token in Integrations.")?;
    // The stored credential may be a JSON field map (bot_token + app_token).
    let bot = serde_json::from_str::<serde_json::Value>(&token)
        .ok()
        .and_then(|v| v.get("bot_token").and_then(|b| b.as_str()).map(String::from))
        .unwrap_or(token);
    let resp = crate::core::egress::client("telegram")
        .post("https://slack.com/api/chat.postMessage")
        .header("Authorization", format!("Bearer {bot}"))
        .json(&serde_json::json!({ "channel": channel, "text": text }))
        .send()
        .await
        .map_err(|e| format!("Slack request failed: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if v["ok"].as_bool().unwrap_or(false) {
        Ok(format!("Sent to Slack ({channel})."))
    } else {
        Err(format!("Slack rejected the message: {}", v["error"].as_str().unwrap_or("unknown error")))
    }
}

/// Post to a Discord channel id (the bot must be in that server).
pub(super) async fn send_discord(channel_id: &str, text: &str) -> Result<String, String> {
    let token = secrets::get_integration_token("discord")
        .ok_or("Discord isn't connected — add its bot token in Integrations.")?;
    let resp = crate::core::egress::client("webhook")
        .post(format!("https://discord.com/api/v10/channels/{channel_id}/messages"))
        .header("Authorization", format!("Bot {token}"))
        .json(&serde_json::json!({ "content": text }))
        .send()
        .await
        .map_err(|e| format!("Discord request failed: {e}"))?;
    if resp.status().is_success() {
        Ok(format!("Sent to Discord ({channel_id})."))
    } else {
        let t = resp.text().await.unwrap_or_default();
        Err(format!("Discord rejected the message: {}", t.chars().take(160).collect::<String>()))
    }
}

/// Send a WhatsApp Cloud API text message to a recipient's phone number.
/// The stored credential carries the access token + the sending phone_number_id.
pub(super) async fn send_whatsapp(to: &str, text: &str) -> Result<String, String> {
    let raw = secrets::get_integration_token("whatsapp")
        .ok_or("WhatsApp isn't connected — add its access token and phone number id in Integrations.")?;
    let creds: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({ "token": raw }));
    let token = creds["token"].as_str().or_else(|| creds["access_token"].as_str()).unwrap_or_default();
    let phone_id = creds["phone_number_id"].as_str().unwrap_or_default();
    if token.is_empty() || phone_id.is_empty() {
        return Err("WhatsApp needs both an access token and a phone number id.".into());
    }
    let v: serde_json::Value = crate::core::egress::client("slack")
        .post(format!("https://graph.facebook.com/v21.0/{phone_id}/messages"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "text",
            "text": { "body": text }
        }))
        .send()
        .await
        .map_err(|e| format!("WhatsApp request failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if v["messages"].is_array() {
        Ok(format!("Sent to WhatsApp ({to})."))
    } else {
        Err(format!("WhatsApp rejected the message: {}", v["error"]["message"].as_str().unwrap_or("unknown error")))
    }
}
