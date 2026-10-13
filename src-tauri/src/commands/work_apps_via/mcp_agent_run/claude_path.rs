use super::*;
use serde_json::json;
use std::sync::atomic::Ordering;

/// A Claude model, with native tool use.
#[allow(clippy::too_many_arguments)]
pub(super) async fn claude_path(app: &tauri::AppHandle, state: &AppState, task: &String, model: &String, system: &String, apps: &mut Apps, http: &crate::core::egress::Client, max_steps: u32) -> Result<String, String> {
    let emit = |kind: &str, text: String| mcp_event(app, kind, text);
    let Apps { sessions, registry, claude_tools, .. } = apps;
    let key = secrets::get_cloud_key()
        .ok_or("Acting via MCP with a Claude model needs an API key — add one in Settings → AI Models, or pick a local model that supports tools (e.g. qwen2.5).")?;
    let mut messages: Vec<serde_json::Value> = vec![json!({ "role": "user", "content": task })];
    for _ in 0..max_steps {
        if state.cu_cancel.load(Ordering::SeqCst) { emit("stopped", "Stopped.".into()); return Ok("Stopped.".into()); }
        // One client — see `core::llm`. This loop had no retry and checked cancel only
        // between steps; both come with the move.
        let req = crate::core::llm::LlmRequest::new(model, 2000)
            .system(system.clone())
            .tools(claude_tools.clone())
            .messages(messages.clone());
        let Some(reply) = crate::core::llm::call(&req, &key, &state.cu_cancel).await? else {
            emit("stopped", "Stopped.".into());
            return Ok("Stopped.".into());
        };
        let content = reply.content.clone();
        let stop_reason = reply.stop_reason.as_str();
        let mut tool_uses = Vec::new();
        for block in &content {
            match block["type"].as_str() {
                Some("text") => { let t = block["text"].as_str().unwrap_or("").trim().to_string(); if !t.is_empty() { emit("text", t); } }
                Some("tool_use") => tool_uses.push(block.clone()),
                _ => {}
            }
        }
        messages.push(json!({ "role": "assistant", "content": content.clone() }));
        if tool_uses.is_empty() || stop_reason == "end_turn" {
            let final_text = content.iter().filter_map(|b| if b["type"]=="text" { b["text"].as_str() } else { None }).collect::<Vec<_>>().join("\n");
            emit("done", "Done.".into());
            return Ok(if final_text.is_empty() { "Done.".into() } else { final_text });
        }
        let mut results = Vec::new();
        for tu in &tool_uses {
            if state.cu_cancel.load(Ordering::SeqCst) { emit("stopped", "Stopped.".into()); return Ok("Stopped.".into()); }
            let id = tu["id"].as_str().unwrap_or("").to_string();
            let name = tu["name"].as_str().unwrap_or("").to_string();
            let input = tu["input"].clone();
            let out = match registry.get(&name) {
                Some((idx, raw)) => {
                    emit("action", format!("{name}"));
                    match sessions[*idx].call_tool(&http, raw, input).await {
                        Ok(r) => mcp_result_text(&r),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                None => format!("Unknown tool: {name}"),
            };
            results.push(json!({ "type": "tool_result", "tool_use_id": id, "content": out }));
        }
        messages.push(json!({ "role": "user", "content": results }));
    }
    emit("done", "Reached the step limit.".into());
    Ok("Reached the step limit — tell me to continue if you'd like.".into())
}
