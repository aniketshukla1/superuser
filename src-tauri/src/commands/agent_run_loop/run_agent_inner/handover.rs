use super::*;
use serde_json::json;

/// Codex: the turn is handed over, not routed.
pub(super) async fn codex(t: &Turn<'_>, history_msgs: Vec<serde_json::Value>) -> Result<String, String> {
    let Turn { app, sink, sys_prompt, message, api_model, working_dir, .. } = t;
    let app = *app;
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    let emit_tool = |name: &str, args: &serde_json::Value| emit_tool_started(app, sink, name, args);
    // ---- Codex: the turn is handed over, not routed ----
    //
    // Everything the loop below does — tools, steps, MCP, skills — is skipped,
    // because Codex runs its own agent and gives back only what it finished
    // with. Said out loud in the transcript rather than left for the user to
    // infer from tools that quietly never fire. `core::codex_cli` has the why.
    emit("text", "Handing this turn to Codex — its own tools and sandbox, not this app's.".into());
    let hist: Vec<(String, String)> = history_msgs
        .iter()
        .filter_map(|m| Some((m.get("role")?.as_str()?.to_string(), m.get("content")?.as_str()?.to_string())))
        .collect();
    let prompt = crate::core::codex_cli::flatten(&sys_prompt.joined(), &hist, &message);
    // Live: each event goes out as Codex produces it, through the same emitter
    // the tool loop uses, so the session socket renders a Codex turn the way it
    // renders any other. A turn at xhigh reasoning runs for minutes, and
    // waiting for the process to exit meant minutes of a blank panel.
    use crate::core::codex_cli::Update;
    let on = |u: Update| match u {
        Update::Text(t) => emit("text", t),
        // Shown as a step rather than prose: it is Codex touching the machine,
        // and the panel already has a shape for that.
        Update::Command(c) => emit_tool("codex_shell", &json!({ "command": c })),
    };
    match crate::core::codex_cli::run(&api_model, &prompt, &working_dir, on).await {
        // Already emitted on its way past — re-emitting would print it twice.
        Ok(answer) => Ok(answer),
        // Returned rather than swallowed into an answer: a failed turn that
        // reads like a reply is one the caller can't tell from a real one.
        Err(e) => Err(e),
    }
}

/// Claude Code: the same handover as Codex, to the user's own `claude`.
pub(super) async fn claude(t: &Turn<'_>, history_msgs: Vec<serde_json::Value>) -> Result<String, String> {
    let Turn { app, sink, sys_prompt, message, api_model, working_dir, .. } = t;
    let app = *app;
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    let emit_tool = |name: &str, args: &serde_json::Value| emit_tool_started(app, sink, name, args);
    // ---- Claude Code: the same handover as Codex, to the user's own `claude` ----
    //
    // `core::claude_cli` has the why. Its tools are read-only (Read, Glob, Grep) in
    // this session's folder, and the transcript says the turn left this app's loop.
    emit("text", "Handing this turn to Claude Code — its own read-only tools, not this app's.".into());
    let hist: Vec<(String, String)> = history_msgs
        .iter()
        .filter_map(|m| Some((m.get("role")?.as_str()?.to_string(), m.get("content")?.as_str()?.to_string())))
        .collect();
    let prompt = crate::core::claude_cli::flatten(&sys_prompt.joined(), &hist, &message);
    use crate::core::claude_cli::Update;
    // Its tool calls are shown under the names this app's own steps already use, so
    // the panel describes a Claude Code read the way it describes one of ours.
    let on = |u: Update| match u {
        Update::Text(t) => emit("text", t),
        Update::Tool { name, detail } => match name.as_str() {
            "Read" => emit_tool("read_file", &json!({ "path": detail })),
            "Grep" => emit_tool("grep", &json!({ "pattern": detail })),
            "Glob" => emit_tool("glob", &json!({ "pattern": detail })),
            other => emit_tool(other, &json!({ "target": detail })),
        },
    };
    crate::core::claude_cli::run(&api_model, &prompt, &working_dir, on).await
}
