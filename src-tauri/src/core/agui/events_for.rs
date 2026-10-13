use super::*;

/// One run's identifiers, as the caller gave them.
#[derive(Debug, Clone)]
pub struct Ids {
    pub thread: String,
    pub run: String,
    /// The id every text event of this run carries.
    pub message: String,
}

impl Ids {
    pub fn from_input(input: &Value) -> Self {
        let thread = input["threadId"].as_str().unwrap_or("").trim().to_string();
        let run = input["runId"].as_str().unwrap_or("").trim().to_string();
        let run = if run.is_empty() { format!("run_{}", crate::core::inbox::short_id()) } else { run };
        Ids { message: format!("msg_{run}"), thread, run }
    }
}

/// The message this run is being asked to answer: the last thing a person said.
///
/// Content arrives as a string or as the array-of-parts shape, because both are in the
/// wild; anything that is not text in a part is skipped rather than guessed at.
pub fn last_user_message(input: &Value) -> Option<String> {
    let messages = input["messages"].as_array()?;
    messages
        .iter()
        .rev()
        .find(|m| m["role"].as_str() == Some("user"))
        .map(|m| text_of(&m["content"]))
        .filter(|t| !t.trim().is_empty())
}

/// Everything said before that, as the pairs a run is seeded with.
///
/// User and assistant text only. A tool result without its call is a malformed prompt,
/// and our own history replay makes the same choice for the same reason.
pub fn history(input: &Value) -> Vec<(String, String)> {
    let Some(messages) = input["messages"].as_array() else { return Vec::new() };
    let last_user = messages.iter().rposition(|m| m["role"].as_str() == Some("user"));
    messages
        .iter()
        .take(last_user.unwrap_or(messages.len()))
        .filter_map(|m| {
            let role = m["role"].as_str().unwrap_or("");
            if !matches!(role, "user" | "assistant") {
                return None;
            }
            let text = text_of(&m["content"]);
            (!text.trim().is_empty()).then(|| (role.to_string(), text))
        })
        .collect()
}

pub(super) fn text_of(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| if p["type"].as_str().unwrap_or("text") == "text" { p["text"].as_str() } else { None })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

pub fn run_started(ids: &Ids) -> Value {
    json!({ "type": "RUN_STARTED", "threadId": ids.thread, "runId": ids.run })
}

pub fn run_finished(ids: &Ids) -> Value {
    json!({ "type": "RUN_FINISHED", "threadId": ids.thread, "runId": ids.run })
}

/// A run that ended badly says so as an event.
///
/// A stream that simply stops leaves the surface waiting forever with nothing to show,
/// which is the worst of the available failures: the person sees a spinner and no reason.
pub fn run_error(ids: &Ids, message: &str) -> Value {
    json!({ "type": "RUN_ERROR", "threadId": ids.thread, "runId": ids.run, "message": message })
}

/// One of our run events, as AG-UI events. Empty for the kinds AG-UI has no place for.
///
/// `text_open` is the state this mapping needs and cannot hold: whether a TEXT_MESSAGE has
/// been opened for this run yet. It is the caller's, because the caller is the thing with
/// a lifetime — and a second opinion about it inside here is how a stream ends up with two
/// starts and one end.
pub fn events_for(our: &Value, ids: &Ids, text_open: &mut bool) -> Vec<Value> {
    let kind = our["type"].as_str().unwrap_or("");
    match kind {
        "text" => {
            let delta = our["text"].as_str().unwrap_or("");
            if delta.is_empty() {
                return Vec::new();
            }
            let mut out = Vec::new();
            if !*text_open {
                *text_open = true;
                out.push(json!({ "type": "TEXT_MESSAGE_START", "messageId": ids.message, "role": "assistant" }));
            }
            out.push(json!({ "type": "TEXT_MESSAGE_CONTENT", "messageId": ids.message, "delta": delta }));
            out
        }
        // A step, reported as it happens so the surface can show it. The call has already
        // run here — this is not an offer to run it there.
        "tool" => {
            let id = our["id"].as_str().map(str::to_string).unwrap_or_else(|| format!("call_{}", crate::core::inbox::short_id()));
            let name = our["name"].as_str().unwrap_or("tool");
            let args = our["arguments"].clone();
            vec![
                json!({ "type": "TOOL_CALL_START", "toolCallId": id, "toolCallName": name, "parentMessageId": ids.message }),
                json!({ "type": "TOOL_CALL_ARGS", "toolCallId": id, "delta": if args.is_null() { "{}".to_string() } else { args.to_string() } }),
                json!({ "type": "TOOL_CALL_END", "toolCallId": id }),
            ]
        }
        _ => Vec::new(),
    }
}

/// The end of a run's text, if any was opened.
pub fn text_end(ids: &Ids, text_open: bool) -> Option<Value> {
    text_open.then(|| json!({ "type": "TEXT_MESSAGE_END", "messageId": ids.message }))
}

/// The whole answer as one message, for a run that streamed nothing.
///
/// A run whose model did not stream still produced an answer, and a stream that carried
/// only RUN_STARTED and RUN_FINISHED would show the surface an empty reply.
pub fn whole_message(ids: &Ids, text: &str) -> Vec<Value> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    vec![
        json!({ "type": "TEXT_MESSAGE_START", "messageId": ids.message, "role": "assistant" }),
        json!({ "type": "TEXT_MESSAGE_CONTENT", "messageId": ids.message, "delta": text }),
        json!({ "type": "TEXT_MESSAGE_END", "messageId": ids.message }),
    ]
}

/// One event, framed as server-sent events.
pub fn sse(event: &Value) -> String {
    format!("data: {event}\n\n")
}
