use super::*;

/// One configured server. `config` is kept as free-form JSON on purpose: it is the
/// same object the ecosystem passes around in `mcpServers` blocks, and re-typing it
/// here would mean rejecting a field some server needs for no gain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpEntry {
    pub name: String,
    #[serde(default)]
    pub config: Value,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Whether this server's tools must be approved per call. Defaults ON: a tool
    /// server is arbitrary remote code, and the safe end is where an unset flag lands.
    #[serde(default = "yes")]
    pub requires_approval: bool,
    /// A fingerprint of the tool surface this server presented when it was last
    /// accepted: for each tool, its name, description and input schema.
    ///
    /// The rug pull is the attack this closes. A server behaves while it is being
    /// reviewed, earns approval, and changes afterwards — and MCP has no
    /// content-addressing or version pinning for tool descriptions, so a server
    /// audited at t0 can present a different description at t1 and nothing anywhere
    /// notices. That matters more here than for most clients, because a tool
    /// description goes into the schema of every turn: it is read as trusted context
    /// before the tool is ever called, which is why `absorb_mcp_tools` already
    /// screens descriptions for injection and taints the run.
    ///
    /// Screening catches a hostile description. This catches a *changed* one, hostile
    /// or not, which is the half screening cannot see.
    ///
    /// Empty means "never fingerprinted" — every server configured before this
    /// existed — and is recorded on the next attach rather than treated as a change.
    /// A first sighting is not evidence of tampering.
    #[serde(default)]
    pub tools_fingerprint: String,
}

pub(super) fn yes() -> bool {
    true
}

/// Fingerprint a server's advertised tool surface.
///
/// Sorted by name, so a server that lists its tools in a different order on a
/// different day does not read as a change. Covers exactly the three fields that
/// reach the model — name, description, input schema — and nothing else: a server
/// bumping its own version string is not something to interrupt the user about.
pub fn fingerprint_tools(tools: &[Value]) -> String {
    use sha2::{Digest, Sha256};
    let mut rows: Vec<String> = tools
        .iter()
        .map(|t| {
            format!(
                "{}\u{1}{}\u{1}{}",
                t["name"].as_str().unwrap_or(""),
                t["description"].as_str().unwrap_or(""),
                serde_json::to_string(&t["inputSchema"]).unwrap_or_default()
            )
        })
        .collect();
    rows.sort();
    let mut h = Sha256::new();
    h.update(rows.join("\u{2}").as_bytes());
    format!("{:x}", h.finalize())[..32].to_string()
}

/// What a fresh look at a server's tools means for a fingerprint we already hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCheck {
    /// Nothing on record — first sighting. Record it; do not call it a change.
    FirstSighting,
    /// Same surface as last time.
    Unchanged,
    /// The tools this server advertises are not the ones that were approved.
    Changed,
}

pub fn check_surface(recorded: &str, fresh: &str) -> SurfaceCheck {
    if recorded.trim().is_empty() {
        SurfaceCheck::FirstSighting
    } else if recorded == fresh {
        SurfaceCheck::Unchanged
    } else {
        SurfaceCheck::Changed
    }
}

impl McpEntry {
    /// "http" | "stdio" | "unknown", read from the config the way the ecosystem
    /// writes it: an explicit `type`, else inferred from which field is present.
    pub fn transport(&self) -> String {
        match self.config.get("type").and_then(|v| v.as_str()) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => {
                if self.config.get("url").and_then(|v| v.as_str()).is_some() {
                    "http".into()
                } else if self.config.get("command").is_some() {
                    "stdio".into()
                } else {
                    "unknown".into()
                }
            }
        }
    }

    pub fn url(&self) -> Option<&str> {
        self.config.get("url").and_then(|v| v.as_str())
    }

    pub fn is_oauth(&self) -> bool {
        self.config.get("auth").and_then(|v| v.as_str()) == Some("oauth")
    }

    /// The one word the row leads with. `enabled` loses to nothing — a server the
    /// user switched off should say so rather than report a connection it isn't using.
    pub fn status(&self) -> String {
        if !self.enabled {
            return "disabled".into();
        }
        if self.is_oauth() {
            return if crate::mcp::oauth::has(&self.name) { "connected" } else { "needs_auth" }.into();
        }
        "configured".into()
    }

    pub fn to_row(&self) -> Value {
        json!({
            "name": self.name,
            "enabled": self.enabled,
            "transport": self.transport(),
            "requires_approval": self.requires_approval,
            "status": self.status(),
            "auth": if self.is_oauth() { Some("oauth") } else { None::<&str> },
            "last_error": Value::Null,
            "tool_count": Value::Null,
            "config": self.config.clone(),
            "builtin": false,
        })
    }
}

/// The connectors we ship, rendered in the same shape. They are listed but not
/// editable: removing one would mean removing a code path, not a config entry, so
/// the registry reports them and refuses to mutate them.
pub fn builtin_rows() -> Vec<Value> {
    crate::mcp::connectors::CONNECTORS
        .iter()
        .map(|c| {
            let connected = crate::mcp::oauth::has(c.id);
            json!({
                "name": c.name,
                "enabled": connected,
                "transport": "http",
                "requires_approval": true,
                "status": if connected { "connected" } else { "needs_auth" },
                "auth": "oauth",
                "last_error": Value::Null,
                "tool_count": Value::Null,
                "config": { "type": "http", "url": c.url, "auth": "oauth" },
                "builtin": true,
                "id": c.id,
            })
        })
        .collect()
}

pub struct McpRegistry {
    pub(super) path: PathBuf,
}
