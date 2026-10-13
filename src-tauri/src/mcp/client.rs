//! Minimal MCP client over the Streamable-HTTP transport: initialize (with
//! session handling), tools/list, tools/call. Responses may come back as plain
//! JSON or as an SSE stream — we handle both.

use super::{connectors::Connector, oauth};
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use crate::integrations::SyncOutcome;
use serde_json::{json, Value};

const PROTO: &str = "2025-06-18";

fn extract(body: &str) -> Value {
    let t = body.trim_start();
    if t.starts_with('{') {
        return serde_json::from_str(t).unwrap_or(Value::Null);
    }
    // SSE: take the last data: line that parses as JSON.
    for line in body.lines().rev() {
        if let Some(d) = line.strip_prefix("data:") {
            if let Ok(v) = serde_json::from_str::<Value>(d.trim()) {
                return v;
            }
        }
    }
    Value::Null
}

pub struct Session {
    url: String,
    token: String,
    id: Option<String>,
    seq: i64,
    server_name: Option<String>,
}

impl Session {
    /// Connect + initialize an MCP session against a connector's OAuth token.
    pub async fn open(id: &str, url: &str, client: &crate::core::egress::Client) -> Result<Session, String> {
        let token = oauth::fresh_token(id).await?;
        let mut s = Session { url: url.to_string(), token, id: None, seq: 1, server_name: None };
        let init = s.rpc(client, json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":PROTO,"capabilities":{},"clientInfo":{"name":"Superuser","version":"0.1.0"}}
        }), true).await?;
        s.server_name = init["serverInfo"]["name"].as_str().map(String::from);
        let _ = s.rpc(client, json!({"jsonrpc":"2.0","method":"notifications/initialized"}), false).await;
        Ok(s)
    }

    /// Connect + initialize with no authorization — for user-added MCP servers
    /// (local or self-hosted endpoints that don't gate behind OAuth). Servers
    /// that require sign-in return an error (typically a 401).
    pub async fn open_anon(url: &str, client: &crate::core::egress::Client) -> Result<Session, String> {
        let mut s = Session { url: url.to_string(), token: String::new(), id: None, seq: 1, server_name: None };
        let init = s.rpc(client, json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":PROTO,"capabilities":{},"clientInfo":{"name":"Superuser","version":"0.1.0"}}
        }), true).await?;
        s.server_name = init["serverInfo"]["name"].as_str().map(String::from);
        let _ = s.rpc(client, json!({"jsonrpc":"2.0","method":"notifications/initialized"}), false).await;
        Ok(s)
    }

    /// The server's self-reported name from the initialize handshake, if any.
    pub fn server_name(&self) -> Option<&str> { self.server_name.as_deref() }

    /// List the server's tools (each carries name / description / inputSchema).
    pub async fn list_tools(&mut self, client: &crate::core::egress::Client) -> Result<Vec<Value>, String> {
        self.seq += 1;
        let r = self.rpc(client, json!({"jsonrpc":"2.0","id":self.seq,"method":"tools/list"}), true).await?;
        Ok(r["tools"].as_array().cloned().unwrap_or_default())
    }

    /// Call one tool and return its result (`content` is an array of blocks).
    pub async fn call_tool(&mut self, client: &crate::core::egress::Client, name: &str, args: Value) -> Result<Value, String> {
        self.seq += 1;
        self.rpc(client, json!({
            "jsonrpc":"2.0","id":self.seq,"method":"tools/call",
            "params":{"name":name,"arguments":args}
        }), true).await
    }

    async fn rpc(&mut self, client: &crate::core::egress::Client, body: Value, expect_reply: bool) -> Result<Value, String> {
        let mut req = client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", PROTO)
            .body(body.to_string());
        if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }
        if let Some(sid) = &self.id {
            req = req.header("Mcp-Session-Id", sid.clone());
        }
        let resp = req.send().await.map_err(|e| format!("MCP request failed: {e}"))?;
        if let Some(sid) = resp.headers().get("Mcp-Session-Id").and_then(|v| v.to_str().ok()) {
            self.id = Some(sid.to_string());
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("MCP server returned {status}: {}", text.chars().take(200).collect::<String>()));
        }
        if !expect_reply {
            return Ok(Value::Null);
        }
        let v = extract(&text);
        if let Some(err) = v.get("error") {
            return Err(format!("MCP error: {}", err["message"].as_str().unwrap_or("unknown")));
        }
        Ok(v["result"].clone())
    }
}

/// Connect, initialize, and pull recent content from a connector into the vault.
pub async fn sync(id: &str, connector: &Connector, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let token = oauth::fresh_token(id).await?;
    let client = crate::core::egress::client("mcp");
    let mut s = Session { url: connector.url.to_string(), token, id: None, seq: 1, server_name: None };

    // initialize
    s.rpc(&client, json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"protocolVersion":PROTO,"capabilities":{},"clientInfo":{"name":"Superuser","version":"0.1.0"}}
    }), true).await?;
    // initialized notification
    let _ = s.rpc(&client, json!({"jsonrpc":"2.0","method":"notifications/initialized"}), false).await;

    // discover tools
    let tools = s.rpc(&client, json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}), true).await?;
    let names: Vec<String> = tools["tools"].as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect()).unwrap_or_default();
    if names.is_empty() {
        return Err("Connected, but the server exposed no tools.".into());
    }

    // pick a search-like tool
    let search = names.iter().find(|n| n.to_lowercase().contains("search"))
        .or_else(|| names.iter().find(|n| n.to_lowercase().contains("list")))
        .cloned()
        .ok_or_else(|| format!("Connected ({} tools), but found no search/list tool to pull content.", names.len()))?;

    let result = s.rpc(&client, json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":search,"arguments":{"query":connector.seed_query}}
    }), true).await?;

    // result.content is an array of blocks; collect text.
    let blocks = result["content"].as_array().cloned().unwrap_or_default();
    let mut out = SyncOutcome::default();
    for (i, block) in blocks.iter().enumerate() {
        let text = block["text"].as_str().unwrap_or("");
        if text.trim().is_empty() { continue; }
        // Use a stable-ish first line as a title.
        let title = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("Item").chars().take(80).collect::<String>();
        let filename = format!("{}-{}.md", id, slug(&title, i));
        let existed = memory.vault_path.join(&filename).exists();
        let input = MemoryInput {
            filename: Some(filename),
            title: format!("{}: {}", connector.name, title),
            category: connector.name.to_string(),
            tags: vec![connector.id.to_string(), "mcp".into()],
            summary: format!("via {} MCP", connector.name),
            body: text.chars().take(4000).collect(),
            origin: Origin::External,
            key: None,
            supersedes: None,
        };
        if memory.save(&input).is_ok() && !existed {
            out.created += 1;
            out.titles.push(format!("{}: {}", connector.name, title));
        }
    }
    Ok(out)
}

fn slug(s: &str, i: usize) -> String {
    let base: String = s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    format!("{}-{i}", base.trim_matches('-').chars().take(40).collect::<String>())
}
