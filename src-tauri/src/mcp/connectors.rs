//! Registry of known remote MCP servers (Claude-style connectors). Each is a
//! hosted MCP endpoint we connect to via OAuth — the same way Claude's
//! connectors work, and the same client code works for any of them.

#[derive(Debug, Clone)]
pub struct Connector {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    /// A broad query used to pull recent content during sync.
    pub seed_query: &'static str,
}

pub const CONNECTORS: &[Connector] = &[
    Connector { id: "notion-mcp", name: "Notion", url: "https://mcp.notion.com/mcp", seed_query: "recent" },
    Connector { id: "linear-mcp", name: "Linear", url: "https://mcp.linear.app/mcp", seed_query: "assigned to me" },
];

pub fn find(id: &str) -> Option<&'static Connector> {
    CONNECTORS.iter().find(|c| c.id == id)
}
