//! Real Notion integration via an internal-integration token. Pulls the most
//! recently edited pages/databases the integration can see into the Memory
//! Tree. Idempotent (keyed by Notion object id).

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use serde_json::{json, Value};

/// Extract a human title from a page or database object.
fn title_of(obj: &Value) -> String {
    // Database: { title: [rich_text] }
    if let Some(arr) = obj["title"].as_array() {
        let t: String = arr.iter().filter_map(|r| r["plain_text"].as_str()).collect();
        if !t.is_empty() {
            return t;
        }
    }
    // Page: find the property of type "title"
    if let Some(props) = obj["properties"].as_object() {
        for v in props.values() {
            if v["type"] == "title" {
                if let Some(arr) = v["title"].as_array() {
                    let t: String = arr.iter().filter_map(|r| r["plain_text"].as_str()).collect();
                    if !t.is_empty() {
                        return t;
                    }
                }
            }
        }
    }
    "Untitled".into()
}

/// Validate a Notion token and return a label (bot/workspace name).
pub async fn verify(token: &str) -> Result<String, String> {
    let client = crate::core::egress::client("notion");
    let resp = client
        .get("https://api.notion.com/v1/users/me")
        .header("Authorization", format!("Bearer {token}"))
        .header("Notion-Version", "2022-06-28")
        .send()
        .await
        .map_err(|e| format!("Network error reaching Notion: {e}"))?;
    match resp.status().as_u16() {
        200 => {
            let v: Value = resp.json().await.map_err(|e| e.to_string())?;
            let name = v["bot"]["workspace_name"].as_str().or(v["name"].as_str()).unwrap_or("Notion workspace");
            Ok(name.to_string())
        }
        401 => Err("Notion rejected the token (401). Copy the *Internal Integration Secret* from your integration's settings.".into()),
        s => Err(format!("Notion returned status {s}.")),
    }
}

pub async fn sync(token: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let client = crate::core::egress::client("notion");
    let resp = client
        .post("https://api.notion.com/v1/search")
        .header("Authorization", format!("Bearer {token}"))
        .header("Notion-Version", "2022-06-28")
        .json(&json!({
            "page_size": 25,
            "sort": { "direction": "descending", "timestamp": "last_edited_time" }
        }))
        .send()
        .await
        .map_err(|e| format!("Network error reaching Notion: {e}"))?;

    match resp.status().as_u16() {
        200 => {}
        401 => return Err("Notion rejected the token (401). Check the integration token.".into()),
        s => return Err(format!("Notion returned status {s}. Make sure pages are shared with your integration.")),
    }

    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let mut out = SyncOutcome::default();

    if let Some(results) = v["results"].as_array() {
        for obj in results {
            let id = obj["id"].as_str().unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let kind = obj["object"].as_str().unwrap_or("page");
            let title = title_of(obj);
            let url = obj["url"].as_str().unwrap_or("");
            let edited = obj["last_edited_time"].as_str().unwrap_or("");

            let filename = format!("notion-{}.md", id.replace('-', ""));
            let existed = memory.vault_path.join(&filename).exists();
            let input = MemoryInput {
                filename: Some(filename),
                title: format!("Notion {kind}: {title}"),
                category: "Notion".into(),
                tags: vec!["notion".into(), kind.into()],
                summary: format!("Last edited {edited}"),
                body: format!("**{title}** ({kind})\n\n- Link: {url}\n- Last edited: {edited}"),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() && !existed {
                out.created += 1;
                out.titles.push(input_title(&title, kind));
            }
        }
    }
    Ok(out)
}

fn input_title(title: &str, kind: &str) -> String {
    format!("Notion {kind} updated: {title}")
}
