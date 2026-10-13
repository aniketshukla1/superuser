use super::*;

/// How long a remote agent has to start answering.
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long one whole run may take.
pub(super) const RUN_TIMEOUT: Duration = Duration::from_secs(300);

/// How many redirects are followed, each re-checked.
pub(super) const MAX_HOPS: usize = 3;

/// Why this address cannot be a remote agent, or `None` if it can.
///
/// The same floor a connected app's server meets — https, no credentials in the address,
/// nothing on this machine or its network, a name a person can check — because it is the
/// same question: everything this app holds is on the other end of it.
pub fn endpoint_problem(url: &str) -> Option<String> {
    crate::core::contentgov::endpoint_problem(url, &crate::core::contentgov::REMOTE_AGENT)
}

/// What a remote agent said, and what it did on the way.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Answer {
    pub text: String,
    /// The tool calls it reported making, by name. Its own, run on its own machine.
    pub steps: Vec<String>,
    /// Set when the run ended with an error event rather than a finish.
    pub error: Option<String>,
}

/// The longest answer read from a remote agent. Past this the stream is cut and the run
/// reads as unfinished: an agent that talks without end is not one whose answer to trust.
pub const MAX_ANSWER_CHARS: usize = 100_000;

/// Reads AG-UI events into one answer, one event at a time, so the same decisions serve a
/// whole body read after the fact and a stream shown as it arrives.
#[derive(Debug, Default)]
pub struct Reader {
    pub(super) answer: Answer,
    pub(super) finished: bool,
    pub(super) cut: bool,
}

impl Reader {
    /// Take one event. Returns false once nothing more should be read.
    pub fn feed(&mut self, event: &Value) -> bool {
        match event["type"].as_str().unwrap_or("") {
            "TEXT_MESSAGE_CONTENT" => {
                self.answer.text.push_str(event["delta"].as_str().unwrap_or(""));
                if self.answer.text.chars().count() > MAX_ANSWER_CHARS {
                    self.cut = true;
                    return false;
                }
            }
            "TOOL_CALL_START" => {
                if let Some(name) = event["toolCallName"].as_str() {
                    self.answer.steps.push(name.to_string());
                }
            }
            "RUN_ERROR" => {
                self.answer.error = Some(event["message"].as_str().unwrap_or("that run failed").to_string());
                return false;
            }
            "RUN_FINISHED" => {
                self.finished = true;
                return false;
            }
            _ => {}
        }
        true
    }

    pub fn finish(mut self) -> Answer {
        if self.cut {
            self.answer.error = Some(format!("that agent's answer ran past {MAX_ANSWER_CHARS} characters and was cut off"));
        }
        // A stream that stops mid-run is not an answer, however much text arrived: the
        // surface has to know the difference between "it finished" and "the connection went".
        if self.answer.error.is_none() && !self.finished {
            self.answer.error = Some("the connection ended before that run finished".into());
        }
        self.answer
    }
}

/// The events in one block of an SSE stream. Multi-line `data:` fields are joined with
/// newlines, as the format says; comments and other fields are ignored.
pub(super) fn events_in(block: &str) -> Option<Value> {
    let data: Vec<&str> = block
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(|d| d.strip_prefix(' ').unwrap_or(d))
        .collect();
    if data.is_empty() {
        return None;
    }
    serde_json::from_str(&data.join("\n")).ok()
}

/// Read an AG-UI event stream into one answer.
pub fn read_stream(body: &str) -> Answer {
    let mut reader = Reader::default();
    for block in body.replace("\r\n", "\n").split("\n\n") {
        if let Some(event) = events_in(block) {
            if !reader.feed(&event) {
                break;
            }
        }
    }
    reader.finish()
}

/// The body a run is asked for in.
pub fn input_for(thread: &str, run: &str, history: &[(String, String)], message: &str) -> Value {
    let mut messages: Vec<Value> = history
        .iter()
        .map(|(role, text)| json!({ "id": format!("m_{}", crate::core::inbox::short_id()), "role": role, "content": text }))
        .collect();
    messages.push(json!({ "id": format!("m_{}", crate::core::inbox::short_id()), "role": "user", "content": message }));
    json!({
        "threadId": thread,
        "runId": run,
        "messages": messages,
        // We offer it nothing to run: a remote agent's tools are its own, and handing it
        // ours would be handing this machine to something we do not govern.
        "tools": [],
        "context": [],
        "state": {},
        "forwardedProps": {}
    })
}

/// What a run's provider is recorded as, so anything downstream — a handoff relaying the
/// answer, the trail — can tell a remote agent's words from a model's here.
pub fn provider_label(endpoint: &str) -> String {
    format!("{PROVIDER_PREFIX}{}", crate::core::boundaries::host_of(endpoint))
}

pub const PROVIDER_PREFIX: &str = "agui:";

pub fn is_remote(provider: &str) -> bool {
    provider.starts_with(PROVIDER_PREFIX)
}

/// Ask a remote agent to run, handing each event to `on_event` as it arrives.
///
/// `http` must be a [`super::egress::client_no_follow`] client, or reqwest follows the
/// redirects below before this function sees them. They are followed by hand, up to
/// [`MAX_HOPS`], and each hop meets the endpoint floor again — a checked address that
/// redirects to an unchecked one is how a floor gets walked around. A hop that changes
/// host drops the credential: it was issued for the first one.
pub async fn run(
    http: &crate::core::egress::Client,
    endpoint: &str,
    token: Option<&str>,
    input: &Value,
    on_event: &mut (dyn FnMut(&Value) + Send),
) -> Result<Answer, String> {
    run_under(http, endpoint, token, input, endpoint_problem, on_event).await
}
