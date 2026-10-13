use super::super::*;

mod apps;
mod ollama_path;
mod claude_path;

use apps::*;
use ollama_path::*;
use claude_path::*;

/// Drive the user's *connected* MCP apps (Notion, Linear, …) to complete a task
/// by letting the model call their tools. Routes by model: Claude uses native
/// tool use; an Ollama model uses Ollama's tool-calling. Streams `mcp-agent`
/// events and stops on `cu_stop`. No screen/mouse — pure structured API calls.
#[tauri::command]
pub async fn mcp_agent_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task: String,
    model: Option<String>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;

    let emit = |kind: &str, text: String| mcp_event(&app, kind, text);

    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| state.settings.lock().unwrap().cloud_model.clone());
    state.cu_cancel.store(false, Ordering::SeqCst);

    let http = crate::core::egress::client("mcp");

    let mut apps = connect_apps(&app, &http).await;

    if apps.sessions.is_empty() {
        return Err("__no_apps__".into());
    }
    emit("text", format!("Connected: {}.", apps.connected_names.join(", ")));

    let system = format!(
        "You complete the user's task by calling tools from their connected apps ({}). \
         Use the tools to actually do the work; don't just describe it.\n\n\
         CRITICAL — never invent or guess identifiers. IDs and URLs (page id, database id, \
         data_source_id, issue id, user id, team id, etc.) must come from a real tool result, \
         not from memory or examples. If a tool needs an ID you don't already have, FIRST call a \
         search / list / query tool to obtain the real value, then pass that exact value. Never use \
         placeholder or example values like 'def456' or 'abc123'. If a call fails with an \
         invalid-identifier error, do not retry with another guess — call a discovery tool to find \
         the correct identifier first.\n\n\
         Read each tool result before the next step. Ask before anything destructive or irreversible. \
         When finished, briefly summarize what you actually did.",
        apps.connected_names.join(", ")
    );
    let max_steps = 40; // each step = one model turn (may carry several tool calls)


    // ---------- Local (Ollama) tool-calling path ----------
    if !model.starts_with("claude") {
        return ollama_path(&app, &state, &task, &model, &system, &mut apps, &http, max_steps).await;
    }

    // ---------- Claude (native tool use) path ----------
    claude_path(&app, &state, &task, &model, &system, &mut apps, &http, max_steps).await
}
