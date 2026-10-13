use super::*;

/// One message worth acting on.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Incoming {
    pub chat_id: String,
    /// Display name of the sender, for the transcript.
    pub from: String,
    pub text: String,
    pub update_id: i64,
}

pub fn bot_token() -> Option<String> {
    secrets::get_integration_token("telegram").filter(|t| !t.trim().is_empty())
}

/// Chats allowed to drive the agent: the configured home chat, plus anything in
/// `telegram_allowed`. Comma-separated so it stays a plain settings field.
pub fn allowed_chats(settings: &crate::core::settings::Settings) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in [settings.telegram_chat_id.as_str(), settings.telegram_allowed.as_str()] {
        for id in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if !out.contains(&id.to_string()) {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// Add a chat to the allowlist. Returns false if it was already there.
///
/// The caller saves the settings; this only edits them, so a handler can validate,
/// mutate and persist in one place rather than each caller inventing the
/// comma-separated splicing again.
pub fn allow(settings: &mut crate::core::settings::Settings, chat_id: &str) -> bool {
    let chat_id = chat_id.trim();
    if chat_id.is_empty() || is_allowed(settings, chat_id) {
        return false;
    }
    let mut list: Vec<String> = settings
        .telegram_allowed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    list.push(chat_id.to_string());
    settings.telegram_allowed = list.join(",");
    true
}

/// Remove a chat from the allowlist.
///
/// The home chat is refused rather than silently ignored: it is where the app SENDS
/// (briefings, automation output), so dropping it here would half-disconnect a
/// channel the user configured somewhere else, and they would find out by not
/// receiving something.
pub fn disallow(settings: &mut crate::core::settings::Settings, chat_id: &str) -> Result<bool, String> {
    let chat_id = chat_id.trim();
    if !chat_id.is_empty() && settings.telegram_chat_id.trim() == chat_id {
        return Err("that's your home chat — change it in Settings ▸ Connectors if you want it gone".into());
    }
    let before: Vec<String> = settings
        .telegram_allowed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let after: Vec<String> = before.iter().filter(|c| c.as_str() != chat_id).cloned().collect();
    let changed = after.len() != before.len();
    settings.telegram_allowed = after.join(",");
    Ok(changed)
}

/// Is this chat allowed to talk to the agent?
///
/// An empty allowlist means **nobody**, deliberately. The alternative — "no list
/// configured, so serve everyone" — turns a bot token pasted into settings into an
/// open door to the user's machine.
pub fn is_allowed(settings: &crate::core::settings::Settings, chat_id: &str) -> bool {
    allowed_chats(settings).iter().any(|c| c == chat_id)
}

pub(super) fn offset_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("telegram-offset")
}

pub(super) fn load_offset(data_dir: &Path) -> i64 {
    std::fs::read_to_string(offset_path(data_dir)).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

pub(super) fn save_offset(data_dir: &Path, offset: i64) {
    // Atomic: a torn offset reads back as 0, and the bot replays every message
    // it has ever seen.
    let _ = crate::core::store::write_atomic(&offset_path(data_dir), &offset.to_string());
}

/// Pull new messages. The offset advances past everything Telegram returned —
/// including messages from chats we refuse — so a stranger messaging the bot can't
/// wedge the queue and stop the owner's messages getting through.
pub async fn poll(data_dir: &Path, settings: &crate::core::settings::Settings) -> Result<Vec<Incoming>, String> {
    let token = bot_token().ok_or("Telegram isn't connected.")?;
    let offset = load_offset(data_dir);
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");

    let resp = crate::core::egress::client("telegram")
        .get(&url)
        // `offset` acknowledges everything before it; `timeout=0` keeps this a
        // short poll so the app's shutdown isn't held open by a long one.
        .query(&[("offset", (offset + 1).to_string()), ("timeout", "0".into()), ("limit", "20".into())])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("Telegram unreachable: {e}"))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["ok"] != true {
        return Err(format!("Telegram: {}", body["description"].as_str().unwrap_or("request failed")));
    }

    let mut out = Vec::new();
    let mut newest = offset;
    for u in body["result"].as_array().cloned().unwrap_or_default() {
        let update_id = u["update_id"].as_i64().unwrap_or(0);
        newest = newest.max(update_id);
        let Some(msg) = u.get("message") else { continue };
        let chat_id = msg["chat"]["id"].as_i64().map(|i| i.to_string()).unwrap_or_default();
        let text = msg["text"].as_str().unwrap_or("").trim().to_string();
        if text.is_empty() {
            continue;
        }
        if !is_allowed(settings, &chat_id) {
            // Silence is the right answer to the stranger and the wrong answer to
            // the owner: this used to `continue` and that was the end of it, so the
            // sender waited forever and nobody here ever learned that someone had
            // tried. Recorded for the Inbox; still not replied to, and deliberately
            // never shown to the agent (see `core::unrouted`).
            let from = msg["from"]["first_name"].as_str().unwrap_or("someone");
            crate::core::unrouted::Unrouted::load(data_dir).record(
                "telegram",
                &chat_id,
                &format!("{from} ({chat_id})"),
                &text,
                "This chat isn't on your Telegram allowlist, so nothing was sent back.",
            );
            continue;
        }
        let from = msg["from"]["first_name"].as_str().unwrap_or("you").to_string();
        out.push(Incoming { chat_id, from, text, update_id });
    }
    if newest > offset {
        save_offset(data_dir, newest);
    }
    Ok(out)
}

/// The session a Telegram chat maps to. Stable per chat, so the conversation
/// continues across messages instead of starting fresh each time — and it shows up
/// in the app's sidebar as an ordinary conversation you can read and carry on.
pub fn session_id(chat_id: &str) -> String {
    format!("telegram-{chat_id}")
}
