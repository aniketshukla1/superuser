//! Custom plugins (user-added MCP servers)


#[derive(serde::Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    /// Coarse risk class inferred from the tool's name + description:
    /// "read" | "write" | "exec" | "external". A hint for permissioning, not a
    /// security guarantee.
    pub risk: String,
}

#[derive(serde::Serialize)]
pub struct McpProbeOut {
    /// The server's self-reported name from the initialize handshake, if any.
    pub server: Option<String>,
    pub tools: Vec<McpToolInfo>,
}

/// Classify a tool's risk from its name + description. Ordered most- to
/// least-dangerous so a tool that both reads and executes lands on "exec".
pub(super) fn classify_tool_risk(name: &str, desc: &str) -> &'static str {
    let h = format!("{name} {desc}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| h.contains(w));
    if has(&["exec", "shell", "command", "spawn", "bash", "terminal", "eval", "run_", "process"]) {
        "exec"
    } else if has(&["http", "fetch", " url", "web", "browse", "download", "scrape", "request", " api", "email", "mail", "message", "send", "sms", "webhook", "publish", "notify"]) {
        "external"
    } else if has(&["write", "create", "update", "delete", "remove", "edit", "insert", "put", "patch", "upload", "save", "modify", "append", "rename", "drop"]) {
        "write"
    } else {
        "read"
    }
}

/// Probe a user-added MCP server: open an anonymous session, run the initialize
/// handshake, and list its tools (each with a coarse risk hint). This is how
/// "connect your own plugin" verifies a server and shows what it can do before
/// the user grants permissions.
#[tauri::command]
pub async fn plugin_mcp_probe(url: String) -> Result<McpProbeOut, String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Enter an http(s) MCP endpoint URL. Local command (stdio) servers aren't supported yet.".into());
    }
    let client = crate::core::egress::client("mcp");
    let mut session = crate::mcp::client::Session::open_anon(url, &client).await.map_err(|e| {
        let low = e.to_lowercase();
        if low.contains("401") || low.contains("unauthorized") || low.contains("403") {
            "This server requires sign-in. Connect authenticated servers from the Integrations tab instead.".to_string()
        } else {
            e
        }
    })?;
    let server = session.server_name().map(String::from);
    let raw = session.list_tools(&client).await?;
    if raw.is_empty() {
        return Err("Connected, but this server exposes no tools.".into());
    }
    let tools = raw.iter().map(|t| {
        let name = t["name"].as_str().unwrap_or("").to_string();
        let description = t["description"].as_str().unwrap_or("").to_string();
        let risk = classify_tool_risk(&name, &description).to_string();
        McpToolInfo { name, description, risk }
    }).collect();
    Ok(McpProbeOut { server, tools })
}
