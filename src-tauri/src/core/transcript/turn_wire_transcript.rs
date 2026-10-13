use super::*;

impl TurnWire {
    /// Capture this turn's own traffic — everything after the caller's own message.
    ///
    /// Returns `None` when the turn produced nothing worth replaying: a plain answer
    /// with no tool calls is already fully described by the assistant message the
    /// session stores, and keeping a second copy of it would double every session
    /// file for no gain.
    pub fn capture(shape: WireShape, model: &str, tail: &[Value]) -> Option<Self> {
        if !tail.iter().any(has_calls) {
            return None;
        }
        let wire = TurnWire {
            shape,
            model: model.to_string(),
            messages: tail.to_vec(),
        };
        Some(wire.repair().compact())
    }

    /// Drop a trailing tool call that never got its result.
    ///
    /// A turn can end mid-step — the step limit, Stop, a provider error — leaving an
    /// assistant message whose calls were never answered. Both APIs reject that on
    /// the way back in, so replaying it would turn one turn's interruption into the
    /// next turn's hard failure. Dropping the unanswered call is the only repair that
    /// keeps the rest: inventing a result would be putting words in a tool's mouth.
    pub fn repair(mut self) -> Self {
        while let Some(last) = self.messages.last() {
            if has_calls(last) {
                self.messages.pop();
            } else {
                break;
            }
        }
        // A transcript that is now only tool results has lost the calls they answer
        // and cannot be replayed into anything.
        if !self.messages.iter().any(has_calls) {
            self.messages.clear();
        }
        self
    }

    /// Cap what is stored, oldest results first.
    ///
    /// Newest-last is deliberate: the most recent tool result is the one the next
    /// turn is most likely to be about, so when something has to go it should be the
    /// output of a command run six steps ago.
    pub fn compact(mut self) -> Self {
        for m in self.messages.iter_mut() {
            cap_result(m, RESULT_CAP);
        }
        while self.byte_len() > TURN_CAP {
            let Some(i) = self.messages.iter().position(is_result) else {
                break;
            };
            // Squeeze rather than remove: dropping a result would orphan the call it
            // answers, which is the invariant `repair` exists to protect.
            if !shrink_result(&mut self.messages[i]) {
                break;
            }
        }
        self
    }

    pub(super) fn byte_len(&self) -> usize {
        self.messages.iter().map(|m| m.to_string().len()).sum()
    }

    /// Whether this transcript can be handed back to `profile`'s model as-is.
    pub fn replays_natively_into(&self, profile: &ModelProfile) -> bool {
        !self.messages.is_empty() && self.shape == WireShape::of(profile)
    }

    /// A labelled text rendering, for replay into a wire shape that is not this one.
    ///
    /// Carries the *content* of what came back, not just the fact that something did.
    /// "read_file → 412 chars" does not stop a model re-reading the file; the first
    /// part of the file does. The label says plainly that this is a summary of the
    /// model's own earlier actions, so it is never mistaken for a tool result
    /// arriving now.
    pub fn project(&self) -> String {
        let acts = self.acts();
        if acts.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "[What I did earlier in this conversation, replayed as a summary because \
             the conversation has since moved to a different model. These are actions \
             already taken — treat their results as known, and do not repeat them.]\n",
        );
        for a in acts {
            out.push_str(&format!("\n{}({})\n→ {}\n", a.name, a.args, a.result));
        }
        out
    }

    /// The calls and results in this transcript, in order, shape-independent.
    pub(super) fn acts(&self) -> Vec<Act> {
        let mut acts: Vec<Act> = Vec::new();
        let mut pending: Vec<(String, String, String)> = Vec::new(); // id, name, args
        for m in &self.messages {
            match self.shape {
                WireShape::Claude => {
                    for b in m["content"].as_array().into_iter().flatten() {
                        match b["type"].as_str() {
                            Some("tool_use") => pending.push((
                                b["id"].as_str().unwrap_or_default().to_string(),
                                b["name"].as_str().unwrap_or("tool").to_string(),
                                brief_args(&b["input"]),
                            )),
                            Some("tool_result") => {
                                let id = b["tool_use_id"].as_str().unwrap_or_default();
                                take(&mut pending, id, &mut acts, claude_result_text(b));
                            }
                            _ => {}
                        }
                    }
                }
                WireShape::OpenAi | WireShape::Ollama => {
                    for c in m["tool_calls"].as_array().into_iter().flatten() {
                        pending.push((
                            c["id"].as_str().unwrap_or_default().to_string(),
                            c["function"]["name"].as_str().unwrap_or("tool").to_string(),
                            // OpenAI sends arguments as a JSON string and Ollama as an
                            // object. Both reach here; neither should be printed raw.
                            match &c["function"]["arguments"] {
                                Value::String(s) => brief_args(
                                    &serde_json::from_str(s).unwrap_or(Value::String(s.clone())),
                                ),
                                o => brief_args(o),
                            },
                        ));
                    }
                    if m["role"] == "tool" {
                        let id = m["tool_call_id"].as_str().unwrap_or_default();
                        let text = m["content"].as_str().unwrap_or_default().to_string();
                        take(&mut pending, id, &mut acts, text);
                    }
                }
            }
        }
        acts
    }
}

pub(super) struct Act {
    pub(super) name: String,
    pub(super) args: String,
    pub(super) result: String,
}

/// One tool call as an observer sees it: what was called, with what, and what came
/// back — shape-independent, so an eval asserting "did it grep before it read" does
/// not have to know which provider ran the turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracedCall {
    pub name: String,
    pub args: String,
    pub result: String,
}

/// The tool calls in a turn, in the order they were made.
///
/// `acts` already computes exactly this for the projection; it was private because
/// nothing outside needed it. The scoreboard does: a turn's *answer* says whether it
/// got there, and only the trace says how.
pub fn tool_trace(w: &TurnWire) -> Vec<TracedCall> {
    w.acts()
        .into_iter()
        .map(|a| TracedCall { name: a.name, args: a.args, result: a.result })
        .collect()
}

/// Match a result to its call.
///
/// By id where there is one, and otherwise to the oldest unanswered call — which is
/// Ollama's case, where `{role: "tool"}` carries no `tool_call_id` at all. Results
/// arrive in the order the calls were emitted, so first-unanswered is the right
/// pairing and not a guess.
pub(super) fn take(pending: &mut Vec<(String, String, String)>, id: &str, acts: &mut Vec<Act>, result: String) {
    let at = pending
        .iter()
        .position(|(pid, _, _)| !id.is_empty() && pid == id)
        .unwrap_or(0);
    if pending.is_empty() {
        return;
    }
    let (_, name, args) = pending.remove(at);
    acts.push(Act {
        name,
        args,
        result: crate::core::context::middle_out(result.trim(), 600),
    });
}
