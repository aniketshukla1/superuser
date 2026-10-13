use super::*;
use serde_json::json;
use std::collections::HashMap;

/// The user's connected MCP apps: an open session to each, and their tools in both wire formats.
pub(super) struct Apps {
    pub(super) sessions: Vec<crate::mcp::client::Session>,
    pub(super) registry: HashMap<String, (usize, String)>,
    pub(super) claude_tools: Vec<serde_json::Value>,
    pub(super) ollama_tools: Vec<serde_json::Value>,
    pub(super) connected_names: Vec<String>,
}

pub(super) async fn connect_apps(app: &tauri::AppHandle, http: &crate::core::egress::Client) -> Apps {
    let emit = |kind: &str, text: String| mcp_event(app, kind, text);
    // Open a session to every connected connector and gather its tools.
    let mut sessions: Vec<crate::mcp::client::Session> = Vec::new();
    let mut registry: HashMap<String, (usize, String)> = HashMap::new();
    let mut claude_tools: Vec<serde_json::Value> = Vec::new();
    let mut ollama_tools: Vec<serde_json::Value> = Vec::new();
    let mut connected_names: Vec<String> = Vec::new();

    for connector in crate::mcp::connectors::CONNECTORS {
        if !crate::mcp::oauth::has(connector.id) {
            continue;
        }
        let mut session = match crate::mcp::client::Session::open(connector.id, connector.url, &http).await {
            Ok(s) => s,
            Err(e) => { emit("text", format!("Couldn't connect to {}: {e}", connector.name)); continue; }
        };
        let tools = session.list_tools(&http).await.unwrap_or_default();
        if tools.is_empty() { continue; }
        connected_names.push(connector.name.to_string());
        let short = connector.id.trim_end_matches("-mcp");
        let idx = sessions.len();
        for t in &tools {
            let raw = match t["name"].as_str() { Some(n) => n, None => continue };
            let name = mcp_tool_name(short, raw);
            let desc = t["description"].as_str().unwrap_or("");
            let schema = if t["inputSchema"].is_object() { t["inputSchema"].clone() } else { json!({ "type": "object", "properties": {} }) };
            claude_tools.push(json!({ "name": name, "description": desc, "input_schema": schema }));
            ollama_tools.push(json!({ "type": "function", "function": { "name": name, "description": desc, "parameters": schema } }));
            registry.insert(name, (idx, raw.to_string()));
        }
        sessions.push(session);
    }
    Apps { sessions, registry, claude_tools, ollama_tools, connected_names }
}

/// One `mcp-agent` event to the GUI.
pub(super) fn mcp_event(app: &tauri::AppHandle, kind: &str, text: String) {
    use tauri::Emitter;
    let _ = app.emit("mcp-agent", json!({ "type": kind, "text": text }));
}
