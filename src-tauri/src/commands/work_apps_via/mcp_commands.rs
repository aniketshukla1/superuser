
/// Flatten an MCP tools/call result (`content` blocks) into plain text.
pub(in crate::commands) fn mcp_result_text(result: &serde_json::Value) -> String {
    let blocks = result["content"].as_array().cloned().unwrap_or_default();
    let text: String = blocks
        .iter()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        result.to_string().chars().take(2000).collect()
    } else {
        text.chars().take(4000).collect()
    }
}

/// Tool name the model sees: `<app>_<tool>`, sanitized to a safe charset.
pub(in crate::commands) fn mcp_tool_name(short: &str, raw: &str) -> String {
    let joined = format!("{short}_{raw}");
    let safe: String = joined
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    safe.chars().take(64).collect()
}
