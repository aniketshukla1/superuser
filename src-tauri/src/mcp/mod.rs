//! Remote-MCP connectors — the Claude way. Connect to a hosted MCP server over
//! OAuth, then speak MCP (Streamable HTTP) to pull content into the Memory Tree.

pub mod client;
pub mod connectors;
pub mod oauth;
pub mod registry;
pub mod stdio;

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct McpConnector {
    pub id: String,
    pub name: String,
    pub url: String,
    pub connected: bool,
}

/// Kept for the shared AppState; remote connectors are stateless (creds live in
/// the secret store), so this is just a marker today.
#[derive(Default)]
pub struct McpHost;

impl McpHost {
    pub fn new() -> Self {
        Self
    }

    pub fn list(&self) -> Vec<McpConnector> {
        connectors::CONNECTORS
            .iter()
            .map(|c| McpConnector {
                id: c.id.to_string(),
                name: c.name.to_string(),
                url: c.url.to_string(),
                connected: oauth::has(c.id),
            })
            .collect()
    }
}

/// One open conversation with an MCP server, whichever transport it speaks.
///
/// Callers should not care: a server reached over HTTP and one we launched on
/// stdio answer the same three questions, and forcing every call site to branch on
/// transport is how one of them ends up supporting only the transport its author
/// happened to test.
pub enum McpLink {
    Http(client::Session),
    Stdio(stdio::StdioSession),
}

impl McpLink {
    /// Open a link to a user-registered server, choosing the transport from its
    /// config. HTTP servers are opened authenticated when we hold credentials for
    /// them and anonymously otherwise — plenty need no sign-in, and demanding one
    /// would hide them.
    pub async fn open_entry(
        entry: &registry::McpEntry,
        http: &crate::core::egress::Client,
    ) -> Result<McpLink, String> {
        if let Some(url) = entry.url() {
            let s = if oauth::has(&entry.name) {
                client::Session::open(&entry.name, url, http).await?
            } else {
                client::Session::open_anon(url, http).await?
            };
            return Ok(McpLink::Http(s));
        }
        if stdio::command_line(&entry.config).is_some() {
            return Ok(McpLink::Stdio(stdio::StdioSession::open(&entry.config).await?));
        }
        Err(format!("{} has neither a `url` nor a `command` to reach it by", entry.name))
    }

    /// Open a link to one of the connectors we ship (always HTTP + OAuth).
    pub async fn open_builtin(
        id: &str,
        url: &str,
        http: &crate::core::egress::Client,
    ) -> Result<McpLink, String> {
        Ok(McpLink::Http(client::Session::open(id, url, http).await?))
    }

    pub fn server_name(&self) -> Option<&str> {
        match self {
            McpLink::Http(s) => s.server_name(),
            McpLink::Stdio(s) => s.server_name(),
        }
    }

    pub async fn list_tools(
        &mut self,
        http: &crate::core::egress::Client,
    ) -> Result<Vec<serde_json::Value>, String> {
        match self {
            McpLink::Http(s) => s.list_tools(http).await,
            McpLink::Stdio(s) => s.list_tools().await,
        }
    }

    pub async fn call_tool(
        &mut self,
        http: &crate::core::egress::Client,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self {
            McpLink::Http(s) => s.call_tool(http, name, args).await,
            McpLink::Stdio(s) => s.call_tool(name, args).await,
        }
    }
}
