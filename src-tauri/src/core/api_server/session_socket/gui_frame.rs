use super::super::*;

pub(in crate::core::api_server) fn gui_frame(ev: &Value) -> Option<Value> {
    Some(match ev["type"].as_str().unwrap_or("") {
        "tool" => json!({ "type": "tool_proposed", "data": {
            "name": ev["name"], "arguments": ev["arguments"], "summary": ev["text"],
        }}),
        // The step's other half. Without it the GUI has nothing to settle a step on,
        // so it spins under a finished answer.
        "tool_done" => json!({ "type": "tool_finished", "data": {
            "name": ev["name"], "status": ev["status"], "result_preview": ev["text"],
        }}),
        "action" => json!({ "type": "tool_proposed", "data": { "name": ev["text"], "summary": ev["text"] } }),
        // A question the run is BLOCKED on. It travels through the feed like every
        // other frame so that switching chats and coming back replays it — the run is
        // still waiting, and a question you can no longer see is a turn that looks hung.
        "question" => json!({ "type": "question_requested", "data": { "questions": ev["questions"] } }),
        // A form rides the same rendezvous as a question and answers with the same
        // `question_response` frame; only the card differs, so only the request does.
        "form" => json!({ "type": "form_requested", "data": { "title": ev["title"], "intro": ev["intro"], "fields": ev["fields"] } }),
        // A tool call the run is BLOCKED on until the user decides. Same reasoning
        // as `question` above: it rides the feed so switching chats and coming back
        // replays it, because the run really is still waiting.
        "permission" => json!({ "type": "permission_required", "data": {
            "name": ev["name"], "arguments": ev["arguments"],
            "reason": ev["reason"], "category": ev["category"],
            "standing_target": ev["standing_target"],
        }}),
        "text" => json!({ "type": "assistant_delta", "data": { "text": ev["text"] } }),
        // The model's own reasoning. The GUI already had the renderer; this is the
        // half that was missing.
        "thinking" => json!({ "type": "reasoning_delta", "data": { "text": ev["text"] } }),
        "stopped" => json!({ "type": "interrupted", "data": {} }),
        // The assistant's browser: a screen a navigation kept, and the two things it can
        // be blocked on — the user's hands, or one value it must never see.
        "computer_frame" => json!({ "type": "computer_frame", "data": {
            "persona": ev["persona"], "frame": ev["frame"], "url": ev["url"], "title": ev["title"],
        }}),
        "computer_help" => json!({ "type": "computer_help", "data": {
            "persona": ev["persona"], "reason": ev["reason"], "url": ev["url"],
        }}),
        "computer_secret" => json!({ "type": "computer_secret", "data": { "persona": ev["persona"], "label": ev["label"] } }),
        // Running token total, one per model call, for the status line under the
        // transcript.
        "usage" => json!({
            "type": "usage",
            "data": { "tokens": ev["text"].as_str().and_then(|t| t.parse::<u64>().ok()).unwrap_or(0) }
        }),
        _ => return None,
    })
}
