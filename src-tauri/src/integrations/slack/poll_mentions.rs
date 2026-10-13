use super::*;

pub(super) const API: &str = "https://slack.com/api";

/// The bot token, unwrapping the JSON field-map form if that's how it was stored.
pub fn bot_token() -> Option<String> {
    let raw = secrets::get_integration_token("slack")?;
    let tok = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| v.get("bot_token").and_then(|b| b.as_str()).map(String::from))
        .unwrap_or(raw);
    (!tok.trim().is_empty()).then_some(tok)
}

pub(super) async fn call(token: &str, method: &str, query: &[(&str, String)]) -> Result<Value, String> {
    let resp = crate::core::egress::client("slack")
        .get(format!("{API}/{method}"))
        .bearer_auth(token)
        .query(query)
        .send()
        .await
        .map_err(|e| format!("Network error reaching Slack: {e}"))?;
    let v: Value = resp.json().await.map_err(|e| format!("Slack returned unreadable JSON: {e}"))?;
    if v["ok"].as_bool().unwrap_or(false) {
        Ok(v)
    } else {
        Err(format!("Slack error: {}", v["error"].as_str().unwrap_or("unknown")))
    }
}

/// Confirm the token works; returns "team · bot".
pub async fn verify(token: &str) -> Result<String, String> {
    let v = call(token, "auth.test", &[]).await?;
    let team = v["team"].as_str().unwrap_or("Slack");
    let user = v["user"].as_str().unwrap_or("bot");
    Ok(format!("{team} · {user}"))
}

/// The bot's own user id (so we can spot mentions of it).
pub async fn self_id(token: &str) -> Result<String, String> {
    let v = call(token, "auth.test", &[]).await?;
    v["user_id"].as_str().map(String::from).ok_or_else(|| "Slack didn't return a user id.".into())
}

/// Channels the bot is a member of.
pub async fn channels(token: &str) -> Result<Vec<(String, String)>, String> {
    let v = call(token, "users.conversations", &[("types", "public_channel,private_channel".into()), ("limit", "50".into())]).await?;
    Ok(v["channels"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| Some((c["id"].as_str()?.to_string(), c["name"].as_str().unwrap_or("channel").to_string())))
                .collect()
        })
        .unwrap_or_default())
}

/// One message that mentions the bot.
#[derive(Debug, Clone)]
pub struct Mention {
    pub channel: String,
    pub channel_name: String,
    pub ts: String,
    /// Reply into this thread (the message's own ts when it isn't threaded).
    pub thread_ts: String,
    pub user: String,
    pub text: String,
}

pub(super) fn watermark_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("slack-watermarks.json")
}

pub(super) fn load_watermarks(data_dir: &Path) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(watermark_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

pub(super) fn save_watermarks(data_dir: &Path, m: &serde_json::Map<String, Value>) {
    if let Ok(s) = serde_json::to_string(m) {
        let _ = crate::core::store::write_atomic(&watermark_path(data_dir), &s);
    }
}

/// Poll every joined channel for messages mentioning the bot since the last poll.
/// The watermark advances even when nothing matched, so a message is only ever
/// handled once.
pub async fn poll_mentions(data_dir: &Path) -> Result<Vec<Mention>, String> {
    let token = bot_token().ok_or("Slack isn't connected.")?;
    let me = self_id(&token).await?;
    let needle = format!("<@{me}>");
    let mut marks = load_watermarks(data_dir);
    let mut out = Vec::new();

    for (id, name) in channels(&token).await? {
        let oldest = marks.get(&id).and_then(|v| v.as_str()).unwrap_or("0").to_string();
        let hist = match call(&token, "conversations.history", &[("channel", id.clone()), ("oldest", oldest.clone()), ("limit", "20".into())]).await {
            Ok(v) => v,
            Err(_) => continue, // not a member / rate-limited — skip this channel
        };
        let msgs = hist["messages"].as_array().cloned().unwrap_or_default();
        let mut newest = oldest.clone();
        for m in &msgs {
            let ts = m["ts"].as_str().unwrap_or("").to_string();
            if ts.is_empty() || m["bot_id"].is_string() {
                continue; // ignore our own / other bots' posts
            }
            if ts.parse::<f64>().unwrap_or(0.0) > newest.parse::<f64>().unwrap_or(0.0) {
                newest = ts.clone();
            }
            let text = m["text"].as_str().unwrap_or("");
            if !text.contains(&needle) {
                continue;
            }
            out.push(Mention {
                channel: id.clone(),
                channel_name: name.clone(),
                thread_ts: m["thread_ts"].as_str().unwrap_or(&ts).to_string(),
                ts,
                user: m["user"].as_str().unwrap_or("").to_string(),
                text: text.replace(&needle, "").trim().to_string(),
            });
        }
        if newest != oldest {
            marks.insert(id, Value::String(newest));
        }
    }
    save_watermarks(data_dir, &marks);
    Ok(out)
}

/// Post a reply, threaded under the message that asked.
pub async fn post_reply(channel: &str, thread_ts: &str, text: &str) -> Result<(), String> {
    let token = bot_token().ok_or("Slack isn't connected.")?;
    let v: Value = crate::core::egress::client("slack")
        .post(format!("{API}/chat.postMessage"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "channel": channel, "thread_ts": thread_ts, "text": text }))
        .send()
        .await
        .map_err(|e| format!("Slack request failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if v["ok"].as_bool().unwrap_or(false) {
        Ok(())
    } else {
        Err(format!("Slack rejected the reply: {}", v["error"].as_str().unwrap_or("unknown")))
    }
}

/// Pull recent channel messages into the Memory Tree.
pub async fn sync(memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let token = bot_token().ok_or("Connect Slack first — add its bot token in Integrations.")?;
    let mut out = SyncOutcome::default();
    for (id, name) in channels(&token).await? {
        let hist = match call(&token, "conversations.history", &[("channel", id.clone()), ("limit", "25".into())]).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        for m in hist["messages"].as_array().cloned().unwrap_or_default() {
            let ts = m["ts"].as_str().unwrap_or("");
            let text = m["text"].as_str().unwrap_or("");
            if ts.is_empty() || text.trim().is_empty() {
                continue;
            }
            let filename = format!("slack-{}-{}.md", id, ts.replace('.', "-"));
            let existed = memory.vault_path.join(&filename).exists();
            let title = format!("#{name}: {}", text.chars().take(80).collect::<String>());
            let input = MemoryInput {
                filename: Some(filename),
                title: title.clone(),
                category: "Slack".into(),
                tags: vec!["slack".into(), "message".into()],
                summary: format!("#{name}"),
                body: text.chars().take(2000).collect(),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() && !existed {
                out.created += 1;
                out.titles.push(title);
            }
        }
    }
    Ok(out)
}
