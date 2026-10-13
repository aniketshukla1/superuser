use super::super::*;

/// The serial path: everything, MCP included.
#[allow(clippy::too_many_arguments)]
pub(in crate::commands) async fn run_tool(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    skills: &crate::core::skills::SkillStore,
    mcp: &mut McpBundle,
    http: &crate::core::egress::Client,
    depth: u8,
    name: &str,
    args: &serde_json::Value,
) -> String {
    run_tool_with(app, state, exec, skills, Some(mcp), http, depth, name, args).await
}

/// The parallel path: no bundle, because nothing here is an MCP tool.
#[allow(clippy::too_many_arguments)]
pub(in crate::commands) async fn run_tool_no_mcp(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    skills: &crate::core::skills::SkillStore,
    http: &crate::core::egress::Client,
    depth: u8,
    name: &str,
    args: &serde_json::Value,
) -> String {
    run_tool_with(app, state, exec, skills, None, http, depth, name, args).await
}

/// What a successful call actually looked at, if it looked at anything.
///
/// Only tools that return CONTENT count. `write_file` names a path the run has not
/// read, and `list_dir` proves a file exists without saying a word about what is in
/// it — treating either as grounding would let a chart cite a file the run only ever
/// heard the name of.
pub(in crate::commands) fn read_target(name: &str, args: &serde_json::Value) -> Option<String> {
    let s = |k: &str| args[k].as_str().map(str::to_string).filter(|v| !v.trim().is_empty());
    match name {
        "read_file" | "doc_read" | "slides_read" | "sheet_read" | "sheet_profile" => s("path"),
        "web_fetch" | "browser_open" | "browser_navigate" => s("url"),
        _ => None,
    }
}
