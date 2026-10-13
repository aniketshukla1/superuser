use super::super::*;

/// Sessions + tool defs gathered from the user's connected MCP apps, merged into
/// the coder agent so it can act on external context (issues, docs, messages).
#[derive(Default)]
pub(in crate::commands) struct McpBundle {
    pub(in crate::commands) sessions: Vec<crate::mcp::McpLink>,
    pub(in crate::commands) registry: std::collections::HashMap<String, (usize, String)>,
    pub(in crate::commands) claude_tools: Vec<serde_json::Value>,
    pub(in crate::commands) ollama_tools: Vec<serde_json::Value>,
    pub(in crate::commands) names: Vec<String>,
    /// Tools whose *description* reads like an instruction to the model rather
    /// than a description of the tool. The server that wrote them chose the words,
    /// and they land in every turn's schema without anyone reading them first.
    pub(in crate::commands) suspect_tools: Vec<String>,
    /// Servers whose advertised tools are not the ones that were approved.
    pub(in crate::commands) surface_changed: Vec<String>,
    /// Session indices whose server the user marked "ask before acting". The risk
    /// heuristic reads a tool's NAME, which is a guess about somebody else's
    /// vocabulary; this is the user's own instruction about a server they added,
    /// so it outranks the guess and can only ever narrow it.
    pub(in crate::commands) always_ask: std::collections::HashSet<usize>,
}

impl McpBundle {
    /// What this run can reach in the user's connected apps, and how to treat it.
    ///
    /// The line this replaces named the apps and stopped. That has a cost: an assistant
    /// holding four Drive tools browsed to
    /// drive.google.com instead, because nothing said a website is the wrong door
    /// when the tools are the right one — and here the web tools fetch as nobody, so
    /// the page is a sign-in screen the model then reasons about. The second thing it
    /// adds is what to do when the tools don't cover something: say so, rather than
    /// improvise a route around a missing capability.
    pub(in crate::commands) fn holdings_block(&self, has_web: bool) -> String {
        let mut counts = vec![0usize; self.names.len()];
        for (idx, _) in self.registry.values() {
            if let Some(c) = counts.get_mut(*idx) {
                *c += 1;
            }
        }
        let apps: Vec<String> = self
            .names
            .iter()
            .zip(counts)
            .map(|(n, c)| format!("{n} ({c} tool{})", if c == 1 { "" } else { "s" }))
            .collect();
        let mut s = format!(
            "\n\n## Connected apps you can act in\n\nYou hold tools for these apps, and they act as the user, with the user's own access: {}.\n\n\
             Use those tools for anything about those apps — tickets, issues, pages, documents, messages.",
            apps.join(", ")
        );
        if has_web {
            s.push_str(
                " Don't fetch or open their websites instead: nothing there is signed in as the user, so a page \
                 shows a sign-in screen or nothing, and neither is an answer.",
            );
        }
        s.push_str(
            " If one of these apps is involved and none of its tools covers what you need, say so plainly and name \
             what's missing — don't work around it. Anything that creates or changes something in them needs the \
             user's approval first.",
        );
        s
    }
}

/// Fold one open server's tools into the bundle under a `short_` prefix.
pub(in crate::commands) fn absorb_mcp_tools(b: &mut McpBundle, label: &str, short: &str, link: crate::mcp::McpLink, tools: &[serde_json::Value], always_ask: bool) {
    use serde_json::json;
    if tools.is_empty() {
        return;
    }
    b.names.push(label.to_string());
    let idx = b.sessions.len();
    for t in tools {
        let raw = match t["name"].as_str() { Some(n) => n, None => continue };
        let name = mcp_tool_name(short, raw);
        let desc = t["description"].as_str().unwrap_or("");
        if crate::core::webguard::reads_as_injection(desc) {
            b.suspect_tools.push(name.clone());
        }
        let schema = if t["inputSchema"].is_object() { t["inputSchema"].clone() } else { json!({ "type": "object", "properties": {} }) };
        b.claude_tools.push(json!({ "name": name, "description": desc, "input_schema": schema }));
        b.ollama_tools.push(json!({ "type": "function", "function": { "name": name, "description": desc, "parameters": schema } }));
        b.registry.insert(name, (idx, raw.to_string()));
    }
    if always_ask {
        b.always_ask.insert(idx);
    }
    b.sessions.push(link);
}

/// Open a session to every *connected* connector AND every enabled server in the
/// user's MCP registry, and collect their tools.
///
/// The registry half is why a server the user added was previously manageable but
/// not usable: it could be listed and toggled in the Connectors tab, and the agent
/// still had no way to call it. Best effort throughout — an unconnected app costs
/// nothing, and one server that fails to start must not cost the turn its tools.
pub(in crate::commands) async fn gather_mcp(http: &crate::core::egress::Client, data_dir: &std::path::Path) -> McpBundle {
    let mut b = McpBundle::default();
    for connector in crate::mcp::connectors::CONNECTORS {
        if !crate::mcp::oauth::has(connector.id) { continue; }
        let mut link = match crate::mcp::McpLink::open_builtin(connector.id, connector.url, http).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let tools = link.list_tools(http).await.unwrap_or_default();
        absorb_mcp_tools(&mut b, connector.name, connector.id.trim_end_matches("-mcp"), link, &tools, false);
    }
    for entry in crate::mcp::registry::McpRegistry::new(data_dir).list() {
        if !entry.enabled { continue; }
        // A name collision with a shipped connector would silently shadow it in the
        // tool table, so the later one is skipped rather than allowed to win.
        if b.names.iter().any(|n| n.eq_ignore_ascii_case(&entry.name)) { continue; }
        let mut link = match crate::mcp::McpLink::open_entry(&entry, http).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let tools = link.list_tools(http).await.unwrap_or_default();
        let short = entry.name.clone();
        // The rug pull check. A server behaves while it is reviewed, earns approval,
        // and changes afterwards; MCP pins nothing, so without this a different
        // description at t1 is indistinguishable from the one that was approved at
        // t0 — and a description is read as trusted context before its tool is ever
        // called. A change does not disable the server: it withdraws the standing
        // approval, so every call from it needs a human this run, and it is audited.
        let fresh = crate::mcp::registry::fingerprint_tools(&tools);
        let changed = match crate::mcp::registry::check_surface(&entry.tools_fingerprint, &fresh) {
            crate::mcp::registry::SurfaceCheck::Changed => true,
            // First sighting is recorded, not reported: every server configured
            // before the field existed has an empty one.
            _ => {
                crate::mcp::registry::McpRegistry::new(data_dir).remember_surface(&entry.name, &fresh);
                false
            }
        };
        if changed {
            b.surface_changed.push(entry.name.clone());
        }
        absorb_mcp_tools(&mut b, &entry.name, &short, link, &tools, entry.requires_approval || changed);
    }
    b
}

/// A prior conversation turn passed from the chat panel for multi-turn context.
#[derive(serde::Deserialize)]
pub struct HistMsg {
    pub role: String,
    pub content: String,
    /// This turn's native tool traffic, when the session kept one. `None` for a plain
    /// turn, for a caller with no session behind it, and for every message written
    /// before H2 was fixed — all of which replay as text, exactly as they always did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<crate::core::transcript::TurnWire>,
}

impl HistMsg {
    /// A history entry with no tool traffic behind it.
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        HistMsg { role: role.into(), content: content.into(), wire: None }
    }
}

/// Tokens the parts of a turn that can't be trimmed will occupy — the system
/// prompt and the message the user just sent. Reserved out of the budget before
/// any of it is handed to history, so a long system prompt eats into the history
/// allowance rather than silently overflowing past it.
pub(in crate::commands) fn estimate_fixed(system: &str, message: &str) -> usize {
    crate::core::context::estimate_tokens(system) + crate::core::context::estimate_tokens(message)
}
