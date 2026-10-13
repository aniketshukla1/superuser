use super::super::*;

/// One turn of an assistant whose agent lives somewhere else.
///
/// The conversation goes out, the reply comes back as it is written, and the steps the
/// remote agent reports are shown as steps — marked as its own, because they ran on its
/// machine under its rules. The trail records that a remote agent was asked and what it
/// said it did; it cannot record what it actually did, and says so.
#[allow(clippy::too_many_arguments)]
pub(in crate::commands) async fn run_remote(
    app: &tauri::AppHandle,
    state: &AppState,
    manifest: &crate::core::personas::PersonaManifest,
    endpoint: &str,
    message: &str,
    sid: &str,
    history: Option<&[HistMsg]>,
    working_dir: &std::path::Path,
    sink: &Option<EventSink>,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<AgentRunOut, String> {
    use crate::core::aguiclient;
    use std::sync::atomic::Ordering;
    use tauri::Emitter;
    let emit = |kind: &str, text: String| {
        let payload = serde_json::json!({ "type": kind, "text": text });
        let _ = app.emit("agent-run", payload.clone());
        if let Some(tx) = sink {
            let _ = tx.send(payload);
        }
    };
    let host = crate::core::boundaries::host_of(endpoint);
    let http = crate::core::egress::client_no_follow("remote-agent");
    let token = crate::core::secrets::get_agent_token(&manifest.id);
    let past: Vec<(String, String)> = history
        .unwrap_or(&[])
        .iter()
        .filter(|h| h.role == "user" || h.role == "assistant")
        .map(|h| (h.role.clone(), h.content.clone()))
        .collect();
    let input = aguiclient::input_for(sid, &format!("run_{}", crate::core::inbox::short_id()), &past, message);
    let began = std::time::Instant::now();

    // Its steps, shown as they are reported. Names only: the arguments are the remote
    // agent's business, and echoing them would put whatever it sent into this app's panel
    // unread.
    let mut open_steps: std::collections::HashMap<String, String> = Default::default();
    let mut on_event = |event: &serde_json::Value| match event["type"].as_str().unwrap_or("") {
        "TEXT_MESSAGE_CONTENT" => {
            if let Some(delta) = event["delta"].as_str().filter(|d| !d.is_empty()) {
                emit("text", delta.to_string());
            }
        }
        "TOOL_CALL_START" => {
            let name = format!("{host}: {}", event["toolCallName"].as_str().unwrap_or("a step"));
            if let Some(id) = event["toolCallId"].as_str() {
                open_steps.insert(id.to_string(), name.clone());
            }
            emit_tool_started(app, sink, &name, &serde_json::json!({}));
        }
        "TOOL_CALL_END" => {
            if let Some(name) = event["toolCallId"].as_str().and_then(|id| open_steps.remove(id)) {
                emit_tool_finished(app, sink, &name, "Ran on the remote agent.");
            }
        }
        _ => {}
    };
    let stopped = async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if cancel.load(Ordering::SeqCst) {
                return;
            }
        }
    };
    let outcome = tokio::select! {
        r = aguiclient::run(&http, endpoint, token.as_deref(), &input, &mut on_event) => Some(r),
        _ = stopped => None,
    };
    let seconds = began.elapsed().as_secs_f64();
    let answer = match outcome {
        None => {
            emit("stopped", "Stopped.".into());
            crate::core::audit::log(&state.data_dir, "remote.stopped", sid, &host, "stopped before the remote agent finished", true);
            "Stopped.".to_string()
        }
        Some(Err(e)) => {
            crate::core::audit::log(&state.data_dir, "remote.failed", sid, &host, &e, false);
            return Err(format!("{} runs on a remote agent at {host}, and that run didn't work: {e}", manifest.name));
        }
        Some(Ok(a)) => {
            let steps = if a.steps.is_empty() { "no steps reported".to_string() } else { format!("reported steps: {}", a.steps.join(", ")) };
            crate::core::audit::log(
                &state.data_dir,
                "remote.answered",
                sid,
                &host,
                &format!(
                    "answered in {seconds:.0}s ({} characters; {steps}). What it did on its own machine is not in this trail.",
                    a.text.chars().count()
                ),
                a.error.is_none(),
            );
            match a.error {
                // Half an answer is shown as half an answer, never as the reply.
                Some(e) if a.text.trim().is_empty() => {
                    return Err(format!("{} runs on a remote agent at {host}, and that run ended with: {e}", manifest.name))
                }
                Some(e) => format!("{}\n\n(The remote agent's run ended early: {e})", a.text),
                None => a.text,
            }
        }
    };
    Ok(AgentRunOut {
        answer,
        usage: None,
        measure: Some(TurnMeasure {
            steps: 1,
            seconds,
            cost_usd: None,
            cache_hit_rate: None,
            tool_seconds: 0.0,
            tool_seconds_summed: 0.0,
            tools_parallel: 0,
        }),
        provider: aguiclient::provider_label(endpoint),
        persona: manifest.id.clone(),
        todos: Vec::new(),
        artifacts: Vec::new(),
        charts: Vec::new(),
        wire: None,
        working_dir: working_dir.to_string_lossy().into_owned(),
    })
}
