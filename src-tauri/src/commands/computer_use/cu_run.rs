use super::super::*;

mod ollama_path;
use ollama_path::*;

pub(crate) async fn cu_run(
    app: tauri::AppHandle,
    state: &AppState,
    task: String,
    model: Option<String>,
) -> Result<String, String> {
    use serde_json::json;
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    let emit = |kind: &str, text: String| {
        let _ = app.emit("computer-use", json!({ "type": kind, "text": text }));
    };

    if !crate::core::computer::accessibility_trusted() {
        return Err("__no_access__".into());
    }

    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| state.settings.lock().unwrap().cloud_model.clone());
    state.cu_cancel.store(false, Ordering::SeqCst);

    // ---------- Local (Ollama vision model) path ----------
    if !model.starts_with("claude") {
        return ollama_path(&app, state, &task, &model).await;
    }

    // ---------- Claude (Anthropic computer-use tool) path ----------
    let key = secrets::get_cloud_key()
        .ok_or("Computer control with a Claude model needs an API key — add one in Settings → AI Models, or pick a local vision model.")?;

    // First screenshot sets the coordinate space we declare to the model.
    let mut shot = crate::core::computer::screenshot()?;
    let tools = json!([{
        "type": "computer_20250124",
        "name": "computer",
        "display_width_px": shot.width,
        "display_height_px": shot.height,
        "display_number": 1
    }]);
    let system = "You are operating the user's macOS computer to complete their task. \
        Start by taking a screenshot to see the current screen, then act in small, verified steps, \
        taking a fresh screenshot to confirm each result. Prefer reliable keyboard shortcuts. \
        Never enter passwords, complete CAPTCHAs, or take destructive or irreversible actions; \
        if the task would require one, stop and explain. When finished, stop and briefly summarize \
        what you did.";

    let client = crate::core::egress::client("computer-use");
    let mut messages: Vec<serde_json::Value> = vec![json!({ "role": "user", "content": task })];

    // 25, from 40. OSWorld 2.0's failure study: 66% of the steps in a failed task are
    // spent in a grounding loop, so the back half of a 40-step budget was mostly the
    // same miss repeated. The stall check below is what ends that; this is the
    // ceiling behind it.
    let max_steps = 25;
    let mut cu_recent: Vec<String> = Vec::new();
    for _ in 0..max_steps {
        if state.cu_cancel.load(Ordering::SeqCst) {
            emit("stopped", "Stopped.".into());
            return Ok("Stopped.".into());
        }

        // Through `core::llm`, which brings the retry policy and the cancel race this
        // loop never had — it checked its stop flag only between steps, so Stop during
        // an inference did nothing until the call returned.
        let req = crate::core::llm::LlmRequest::new(&model, 1500)
            .system(system.clone())
            .tools(tools.as_array().cloned().unwrap_or_default())
            .messages(messages.clone())
            .beta("computer-use-2025-01-24");
        let Some(reply) = crate::core::llm::call(&req, &key, &state.cu_cancel).await? else {
            emit("stopped", "Stopped.".into());
            return Ok("Stopped.".into());
        };
        let content = reply.content.clone();
        let stop_reason = reply.stop_reason.as_str();

        let mut tool_uses = Vec::new();
        for block in &content {
            match block["type"].as_str() {
                Some("text") => {
                    let t = block["text"].as_str().unwrap_or("").trim().to_string();
                    if !t.is_empty() {
                        emit("text", t);
                    }
                }
                Some("tool_use") => tool_uses.push(block.clone()),
                _ => {}
            }
        }
        messages.push(json!({ "role": "assistant", "content": content.clone() }));

        if tool_uses.is_empty() || stop_reason == "end_turn" {
            let final_text = content
                .iter()
                .filter_map(|b| if b["type"] == "text" { b["text"].as_str() } else { None })
                .collect::<Vec<_>>()
                .join("\n");
            emit("done", "Task complete.".into());
            return Ok(if final_text.is_empty() { "Done.".into() } else { final_text });
        }

        let mut results = Vec::new();
        for tu in &tool_uses {
            if state.cu_cancel.load(Ordering::SeqCst) {
                emit("stopped", "Stopped.".into());
                return Ok("Stopped.".into());
            }
            let id = tu["id"].as_str().unwrap_or("").to_string();
            let input = &tu["input"];
            let action = input["action"].as_str().unwrap_or("screenshot");
            emit("action", cu_describe(action, input));

            if action == "wait" {
                let secs = input["duration"].as_f64().unwrap_or(1.0).min(5.0);
                std::thread::sleep(std::time::Duration::from_millis((secs * 1000.0) as u64));
            } else if action != "screenshot" && action != "cursor_position" {
                cu_recent.push(cu_signature(&cu_describe(action, input)));
                if cu_stalled(&cu_recent) {
                    emit("done", "Stopped: the same action three times in a row.".into());
                    return Ok(cu_stall_report(&cu_recent));
                }
                let act = cu_to_action(action, input);
                if let Err(e) = crate::core::computer::execute(&act, &shot) {
                    results.push(json!({
                        "type": "tool_result", "tool_use_id": id, "is_error": true,
                        "content": format!("Action failed: {e}")
                    }));
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_millis(450)); // let the UI settle
            }

            shot = crate::core::computer::screenshot()?;
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": id,
                "content": [{
                    "type": "image",
                    "source": { "type": "base64", "media_type": "image/png", "data": shot.base64 }
                }]
            }));
        }
        messages.push(json!({ "role": "user", "content": results }));
    }

    emit("done", "Reached the step limit.".into());
    Ok("Reached the step limit — tell me to continue if you'd like.".into())
}
