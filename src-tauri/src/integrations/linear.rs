//! Linear integration via a personal API key (GraphQL). Pulls the viewer's
//! assigned and recently updated issues into the Memory Tree.

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use serde_json::{json, Value};

const API: &str = "https://api.linear.app/graphql";

async fn gql(client: &crate::core::egress::Client, key: &str, query: &str) -> Result<Value, String> {
    let resp = client
        .post(API)
        .header("Authorization", key)
        .header("Content-Type", "application/json")
        .json(&json!({ "query": query }))
        .send()
        .await
        .map_err(|e| format!("Network error reaching Linear: {e}"))?;
    match resp.status().as_u16() {
        200 => {
            let v: Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(errs) = v["errors"].as_array() {
                if !errs.is_empty() {
                    return Err(format!("Linear API error: {}", errs[0]["message"].as_str().unwrap_or("unknown")));
                }
            }
            Ok(v["data"].clone())
        }
        401 | 400 => Err("Linear rejected the API key. Create a personal API key in Linear → Settings → API.".into()),
        s => Err(format!("Linear returned status {s}.")),
    }
}

pub async fn verify(key: &str) -> Result<String, String> {
    let client = crate::core::egress::client("linear");
    let data = gql(&client, key, "{ viewer { name email } }").await?;
    let name = data["viewer"]["name"].as_str().unwrap_or("Linear user");
    let email = data["viewer"]["email"].as_str().unwrap_or("");
    Ok(if email.is_empty() { name.to_string() } else { format!("{name} ({email})") })
}

pub async fn sync(key: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let client = crate::core::egress::client("linear");
    let data = gql(&client, key,
        "{ issues(first: 40, orderBy: updatedAt) { nodes { id identifier title description updatedAt state { name } assignee { name } team { name } } } }"
    ).await?;

    let mut out = SyncOutcome::default();
    if let Some(nodes) = data["issues"]["nodes"].as_array() {
        for n in nodes {
            let id = n["id"].as_str().unwrap_or("");
            if id.is_empty() { continue; }
            let ident = n["identifier"].as_str().unwrap_or("");
            let title = n["title"].as_str().unwrap_or("Untitled");
            let state = n["state"]["name"].as_str().unwrap_or("");
            let team = n["team"]["name"].as_str().unwrap_or("");
            let assignee = n["assignee"]["name"].as_str().unwrap_or("Unassigned");
            let filename = format!("linear-{}.md", id.replace('-', ""));
            let existed = memory.vault_path.join(&filename).exists();
            let input = MemoryInput {
                filename: Some(filename),
                title: format!("{ident}: {title}"),
                category: "Linear".into(),
                tags: vec!["linear".into(), "issue".into()],
                summary: format!("{state} · {team} · {assignee}"),
                body: format!("**{ident} — {title}**\n\n- State: {state}\n- Team: {team}\n- Assignee: {assignee}\n\n{}", n["description"].as_str().unwrap_or("")),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() && !existed {
                out.created += 1;
                out.titles.push(format!("Linear {ident}: {title}"));
            }
        }
    }
    Ok(out)
}
