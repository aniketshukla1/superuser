use super::*;
use serde_json::json;
use std::sync::atomic::Ordering;

/// A local (Ollama) vision model: it reads each screenshot and answers with one JSON action.
pub(super) async fn ollama_path(app: &tauri::AppHandle, state: &AppState, task: &String, model: &String) -> Result<String, String> {
    use tauri::Emitter;
    let emit = |kind: &str, text: String| {
        let _ = app.emit("computer-use", json!({ "type": kind, "text": text }));
    };
    // 25, from 40. OSWorld 2.0's failure study: 66% of the steps in a failed task are
    // spent in a grounding loop, so the back half of a 40-step budget was mostly the
    // same miss repeated. The stall check below is what ends that; this is the
    // ceiling behind it.
    let max_steps = 25;
    let mut cu_recent: Vec<String> = Vec::new();
    // The capability, not the name: refusing `gemma4:12b` here told a user their
    // model could not see the screen when it can.
    let settings_snapshot = { state.settings.lock().unwrap().clone() };
    let infos = crate::core::ollama::list_model_infos(&settings_snapshot)
        .await
        .unwrap_or_default();
    if !crate::core::ollama::supports_vision(&infos, &model) {
        return Err(format!(
            "“{model}” can't see the screen. Pick a vision model in the model menu — qwen2.5vl is recommended for clicking accuracy."
        ));
    }
    let ollama_url = { state.settings.lock().unwrap().ollama_url.clone() };
    let url = format!("{}/api/chat", ollama_url.trim_end_matches('/'));
    let system = "You operate a macOS computer to complete the user's task by looking at \
        a screenshot and choosing ONE next action. Reply with ONLY a single JSON object, no \
        prose, no code fences. Schema: {\"action\": one of \
        \"left_click\",\"right_click\",\"double_click\",\"type\",\"key\",\"scroll\",\"done\"}. \
        For clicks include integer pixel \"x\" and \"y\" within the screenshot. For \"type\" \
        include \"text\". For \"key\" include \"text\" like \"Return\" or \"cmd+s\". For \
        \"scroll\" include \"x\",\"y\",\"scroll_direction\" (\"up\"/\"down\") and \"scroll_amount\". \
        When the task is complete use {\"action\":\"done\",\"summary\":\"...\"}. Never enter \
        passwords or do destructive actions; if needed, return done with an explanation.";

    let client = crate::core::egress::client("computer-use");
    let mut history: Vec<String> = Vec::new();
    let mut shot = crate::core::computer::screenshot()?;

    for _ in 0..max_steps {
        if state.cu_cancel.load(Ordering::SeqCst) {
            emit("stopped", "Stopped.".into());
            return Ok("Stopped.".into());
        }
        let hist = if history.is_empty() { "None yet.".to_string() } else { history.join("\n") };
        let user = format!(
            "Task: {task}\n\nActions so far:\n{hist}\n\nThis is the current screen ({}x{} pixels). \
             Reply with ONLY the JSON for the single next action.",
            shot.width, shot.height
        );
        let body = json!({
            "model": model,
            "stream": false,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user, "images": [shot.base64] }
            ],
            "options": { "num_ctx": 8192, "num_predict": 300, "temperature": 0 },
            "keep_alive": crate::core::ollama::KEEP_ALIVE
        });
        let resp = client.post(&url).json(&body).send().await
            .map_err(|e| format!("Could not reach Ollama at {ollama_url}. ({e})"))?;
        if !resp.status().is_success() {
            let st = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {st}: {}", t.chars().take(200).collect::<String>()));
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = v["message"]["content"].as_str().unwrap_or("").to_string();
        let act = match cu_extract_json(&content) {
            Some(a) => a,
            None => {
                emit("done", "The model didn't return a usable action.".into());
                return Ok(format!("Stopped — the model replied without a clear action:\n{content}"));
            }
        };
        let action = act["action"].as_str().unwrap_or("done");
        if action == "done" {
            let summary = act["summary"].as_str().unwrap_or("Done.").to_string();
            emit("done", "Task complete.".into());
            return Ok(summary);
        }
        emit("action", cu_local_describe(&act));
        // The grounding loop, caught while it happens rather than at the step
        // limit: the same action at (nearly) the same place three times running.
        cu_recent.push(cu_signature(&cu_local_describe(&act)));
        if cu_stalled(&cu_recent) {
            emit("done", "Stopped: the same action three times in a row.".into());
            return Ok(cu_stall_report(&history));
        }
        let a = cu_local_action(&act);
        if let Err(e) = crate::core::computer::execute(&a, &shot) {
            history.push(format!("{} → failed: {e}", cu_local_describe(&act)));
        } else {
            history.push(cu_local_describe(&act));
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        shot = crate::core::computer::screenshot()?;
    }
    emit("done", "Reached the step limit.".into());
    return Ok("Reached the step limit — tell me to continue if you'd like.".into());
}
