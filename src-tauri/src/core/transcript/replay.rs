use super::*;

/// Tool arguments as one readable line: `path="alpha.md", limit=20`.
pub(super) fn brief_args(v: &Value) -> String {
    let Some(o) = v.as_object() else {
        return String::new();
    };
    o.iter()
        .map(|(k, val)| {
            let shown = match val {
                Value::String(s) => format!("{:?}", crate::core::context::middle_out(s, 80)),
                other => other.to_string(),
            };
            format!("{k}={shown}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn claude_result_text(b: &Value) -> String {
    match &b["content"] {
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Does this message ask for tools? True for both an OpenAI-style `tool_calls` array
/// and a Claude `tool_use` block.
pub(super) fn has_calls(m: &Value) -> bool {
    if m["tool_calls"].as_array().is_some_and(|a| !a.is_empty()) {
        return true;
    }
    m["content"]
        .as_array()
        .is_some_and(|a| a.iter().any(|b| b["type"] == "tool_use"))
}

/// Does this message carry a tool's answer?
pub(super) fn is_result(m: &Value) -> bool {
    if m["role"] == "tool" {
        return true;
    }
    m["content"]
        .as_array()
        .is_some_and(|a| a.iter().any(|b| b["type"] == "tool_result"))
}

pub(super) fn cap_result(m: &mut Value, limit: usize) {
    if m["role"] == "tool" {
        if let Some(s) = m["content"].as_str() {
            if s.len() > limit {
                m["content"] = json!(crate::core::context::middle_out(s, limit));
            }
        }
        return;
    }
    if let Some(blocks) = m["content"].as_array_mut() {
        for b in blocks {
            if b["type"] == "tool_result" {
                if let Some(s) = b["content"].as_str() {
                    if s.len() > limit {
                        b["content"] = json!(crate::core::context::middle_out(s, limit));
                    }
                }
            }
        }
    }
}

/// Halve one result's allowance. Returns false when there is nothing left to take.
pub(super) fn shrink_result(m: &mut Value) -> bool {
    let before = m.to_string().len();
    cap_result(m, 400);
    m.to_string().len() < before
}

/// Assemble the messages a new turn replays, choosing native or projected per turn.
///
/// Takes the history as `(role, content, wire)` and the profile of the model about to
/// run. A stored turn whose shape matches is handed back whole; one that does not is
/// rendered to text and attached to the assistant message it belongs to, so the
/// conversation keeps its shape either way.
pub fn replay(
    history: &[(String, String, Option<TurnWire>)],
    profile: &ModelProfile,
) -> Vec<Value> {
    let mut out = Vec::new();
    for (role, content, wire) in history {
        // The stored traffic belongs to the assistant's half of a turn. A user
        // message is a user message whatever rides beside it.
        let native = role == "assistant"
            && wire.as_ref().is_some_and(|w| w.replays_natively_into(profile) && closes_cleanly(w, content));
        match wire {
            Some(w) if native => {
                out.extend(w.messages.iter().cloned());
                // The final prose, when the stored traffic stops at a tool result —
                // a turn that ended at the step limit, or on Stop. Without it the
                // conversation would put two user messages back to back, which the
                // Anthropic API rejects and which reads to any model as a question
                // nobody answered.
                if !ends_assistant(w) {
                    out.push(json!({ "role": "assistant", "content": content }));
                }
            }
            Some(w) => {
                let projected = w.project();
                let joined = if projected.is_empty() {
                    content.clone()
                } else if content.trim().is_empty() {
                    projected
                } else {
                    format!("{projected}\n{content}")
                };
                if !joined.trim().is_empty() {
                    out.push(json!({ "role": role, "content": joined }));
                }
            }
            // No stored transcript: a plain turn, or one from before this existed.
            None => {
                if !content.trim().is_empty() {
                    out.push(json!({ "role": role, "content": content }));
                }
            }
        }
    }
    out
}

/// Whether replaying this transcript leaves the conversation ending on the
/// assistant, as every turn must.
///
/// It does if the stored traffic already ends there, or if there is an answer to
/// append. Neither — a turn cut short after a tool result, with no prose recorded —
/// is projected instead, because text is well formed whatever it contains.
pub(super) fn closes_cleanly(w: &TurnWire, content: &str) -> bool {
    ends_assistant(w) || !content.trim().is_empty()
}

/// Whether the stored traffic ends on the assistant's side.
///
/// `repair` has already removed any unanswered calls, so an assistant message here
/// is a complete one.
pub(super) fn ends_assistant(w: &TurnWire) -> bool {
    w.messages.last().is_some_and(|m| m["role"] == "assistant")
}
