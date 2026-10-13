use super::*;
use serde_json::json;
use std::sync::atomic::Ordering;

/// A local (Ollama) model, with Ollama's tool-calling.
#[allow(clippy::too_many_arguments)]
pub(super) async fn ollama_path(app: &tauri::AppHandle, state: &AppState, task: &String, model: &String, system: &String, apps: &mut Apps, http: &crate::core::egress::Client, max_steps: u32) -> Result<String, String> {
    let emit = |kind: &str, text: String| mcp_event(app, kind, text);
    let Apps { sessions, registry, ollama_tools, .. } = apps;
    let ollama_url = { state.settings.lock().unwrap().ollama_url.clone() };
    let url = format!("{}/api/chat", ollama_url.trim_end_matches('/'));
    // Small models love to *describe* tool calls in prose instead of emitting
    // real ones — spell out the contract and a DONE marker for completion.
    let local_system = format!(
        "{system}\n\nYou MUST use the tool-calling mechanism to act — never write tool calls, \
         JSON, or code as your text answer, and never invent IDs. To get an ID you don't have, \
         call a search or list tool first and read the real value from its result. \
         Don't over-research: after one or two reads to find the target and its IDs, switch to \
         actually creating/updating the content. Never call the same read tool again and again. \
         Take one step at a time. Only when the entire task is truly finished, reply with the single \
         word DONE on its own line and nothing else."
    );
    let mut messages: Vec<serde_json::Value> = vec![
        json!({ "role": "system", "content": local_system }),
        json!({ "role": "user", "content": task }),
    ];
    let mut nudges = 0;
    let max_nudges = 3;
    let mut read_streak = 0; // consecutive read-only tool calls (stuck-detection)

    // Can this model show its working? Asked once per run, not per step — and
    // checked rather than assumed, because Ollama rejects `think: true` outright
    // for a model without the capability, which would break the turn instead of
    // just omitting a nicety.
    let can_think = match http
        .get(format!("{}/api/tags", ollama_url.trim_end_matches('/')))
        .send()
        .await
    {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(tags) => crate::core::ollama::supports_thinking(
                &crate::core::ollama::parse_model_infos(&tags),
                model.as_ref(),
            ),
            Err(_) => false,
        },
        Err(_) => false,
    };

    for _ in 0..max_steps {
        if state.cu_cancel.load(Ordering::SeqCst) { emit("stopped", "Stopped.".into()); return Ok("Stopped.".into()); }
        let mut body = json!({ "model": model, "stream": false, "messages": messages, "tools": ollama_tools });
        if can_think {
            body["think"] = json!(true);
        }
        crate::core::ollama::stamp_keep_alive(&mut body, &model);
        let resp = http.post(&url).json(&body).send().await
            .map_err(|e| format!("Could not reach Ollama at {ollama_url}. ({e})"))?;
        if !resp.status().is_success() {
            let st = resp.status(); let t = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {st}: {}", t.chars().take(200).collect::<String>()));
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let msg = v["message"].clone();
        // Show the working. The GUI has rendered a `ThinkingBlock` for reasoning
        // models since before this existed — it was simply never sent anything,
        // because nothing on this side ever emitted the event. Without it a local
        // model looks idle for a minute and then produces an answer, and the user
        // has no idea whether it is reasoning, stuck, or about to do something
        // they'd rather it didn't.
        if let Some(t) = msg["thinking"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
            emit("thinking", t.to_string());
        }
        let calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
        messages.push(msg.clone());
        if calls.is_empty() {
            let content = msg["content"].as_str().unwrap_or("").to_string();
            // Genuine completion only when it signals DONE.
            if content.contains("DONE") {
                emit("done", "Done.".into());
                let summary = content.replace("DONE", "").trim().to_string();
                return Ok(if summary.is_empty() { "Done.".into() } else { summary });
            }
            // Otherwise it answered in prose without calling tools — push back.
            if nudges < max_nudges {
                nudges += 1;
                emit("text", "(nudging the model to actually call the tools…)".into());
                messages.push(json!({ "role": "user", "content":
                    "You did not call any tool — you only described one and/or used placeholder IDs. \
                     Do NOT answer in text. Make a real tool call now: start by calling a search/list \
                     tool to get the actual data_source_id, then continue one step at a time. If the \
                     task is truly complete, reply with only: DONE" }));
                continue;
            }
            emit("done", "Couldn't complete with this model.".into());
            return Ok(format!(
                "I couldn't finish this with {model}: it described the steps but never made real \
                 tool calls (and used placeholder IDs). Small local models struggle with multi-step \
                 app actions — try a Claude model, or a larger local model. Its draft plan:\n\n{}",
                content.chars().take(1500).collect::<String>()
            ));
        }
        nudges = 0;
        let mut did_write = false;
        for c in &calls {
            if state.cu_cancel.load(Ordering::SeqCst) { emit("stopped", "Stopped.".into()); return Ok("Stopped.".into()); }
            let name = c["function"]["name"].as_str().unwrap_or("").to_string();
            let lname = name.to_lowercase();
            if ["create", "update", "append", "insert", "add", "write", "delete", "patch", "post", "comment", "move", "new"]
                .iter().any(|w| lname.contains(w)) { did_write = true; }
            let (args, unreadable) = read_arguments(&c["function"]["arguments"]);
            // Same rule as the assistant loop: unreadable arguments come back as
            // this call's result rather than being run as an empty object.
            let out = match (unreadable, registry.get(&name)) {
                (Some(why), _) => why,
                (None, entry) => match entry {
                    Some((idx, raw)) => {
                        emit("action", format!("{name}"));
                        match sessions[*idx].call_tool(&http, raw, args).await {
                            Ok(r) => mcp_result_text(&r),
                            Err(e) => format!("Error: {e}"),
                        }
                    }
                    None => format!("Unknown tool: {name}"),
                },
            };
            messages.push(json!({ "role": "tool", "content": out }));
        }
        // Stuck-reading guard: bail (honestly) if it just keeps searching/fetching.
        if did_write { read_streak = 0; } else { read_streak += calls.len(); }
        if read_streak >= 6 {
            emit("done", "Stuck researching.".into());
            return Ok(format!(
                "{model} kept searching/reading without ever creating anything — small local models \
                 often get stuck like this on multi-step tasks. For real app actions, use a Claude \
                 model. A more specific task also helps (e.g. name the exact page or database to \
                 create the plan in)."
            ));
        }
    }
    emit("done", "Reached the step limit.".into());
    return Ok("Reached the step limit — tell me to continue if you'd like.".into());
}
