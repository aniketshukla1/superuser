use super::*;

/// How many steps after a boundary are watched. The paper's window.
pub const WATCH_STEPS: usize = 3;

/// One compaction event, as it happened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Boundary {
    /// How many messages `trim_turns` dropped.
    pub turns_dropped: usize,
    /// How much text went with them. Size is the honest measure of what a summary
    /// is standing in for.
    pub chars_dropped: usize,
    /// Whether a summary was actually produced. An empty summary means the turns
    /// are simply gone, which is FIFO wearing compaction's name — worth telling
    /// apart, because the paper measured those two arms as very different.
    pub summarized: bool,
}

/// Every `(tool, arguments)` pair in a batch of messages, in both provider shapes.
///
/// Arguments are compared as their serialized form rather than parsed and
/// normalised: two calls that differ only in key order are not what this is looking
/// for, and treating them as identical would report a refetch the agent did not
/// make. A false negative here is cheap; a false positive would put a number in a
/// report that is not true.
pub fn calls_in(msgs: &[Value]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for m in msgs {
        // OpenAI-compatible: assistant message with a `tool_calls` array.
        if let Some(tcs) = m["tool_calls"].as_array() {
            for tc in tcs {
                let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let args = tc["function"]["arguments"].as_str().map(str::to_string).unwrap_or_else(|| tc["function"]["arguments"].to_string());
                if !name.is_empty() {
                    out.push((name, args));
                }
            }
        }
        // Claude native: content blocks of type `tool_use`.
        if let Some(blocks) = m["content"].as_array() {
            for b in blocks {
                if b["type"] == "tool_use" {
                    let name = b["name"].as_str().unwrap_or("").to_string();
                    if !name.is_empty() {
                        out.push((name, b["input"].to_string()));
                    }
                }
            }
        }
    }
    out
}

/// Counts what the agent does in the steps after a boundary.
#[derive(Debug, Clone)]
pub struct Watch {
    pub(super) boundary: Boundary,
    /// Calls that were answered in the turns that got dropped. Asking for one of
    /// these again is the refetch signal.
    pub(super) lost: Vec<(String, String)>,
    pub(super) steps_left: usize,
    pub(super) refetches: usize,
    pub(super) blocked: usize,
    pub(super) steps_seen: usize,
}

impl Watch {
    /// Open a watch over the steps following `boundary`. `dropped` is what
    /// `trim_turns` handed back.
    pub fn open(boundary: Boundary, dropped: &[Value]) -> Self {
        Self {
            boundary,
            lost: calls_in(dropped),
            steps_left: WATCH_STEPS,
            refetches: 0,
            blocked: 0,
            steps_seen: 0,
        }
    }

    /// Record that a summary was produced for this boundary.
    pub fn set_summarized(&mut self, yes: bool) {
        self.boundary.summarized = yes;
    }

    /// Whether this watch is still counting.
    pub fn open_still(&self) -> bool {
        self.steps_left > 0
    }

    /// Record one executed call. Cheap enough to call unconditionally; it does
    /// nothing once the window has closed.
    pub fn observe(&mut self, name: &str, args: &str, result: &str) {
        if self.steps_left == 0 {
            return;
        }
        self.steps_seen += 1;
        if self.lost.iter().any(|(n, a)| n == name && a == args) {
            self.refetches += 1;
        }
        // A refusal, however it is worded. The gates answer in prose, so this reads
        // the shape they all share rather than matching any one gate's sentence.
        let r = result.trim_start();
        if r.starts_with("error:") || r.starts_with("refused") || r.contains("not allowed") {
            self.blocked += 1;
        }
    }

    /// Score a finished turn's messages in one pass.
    ///
    /// Scoring after the fact rather than live, because a live watch would have to
    /// be threaded through the whole dispatch path to count two integers. Everything
    /// it needs is already in the message array the turn built: calls in assistant
    /// messages, results in the `tool` messages that answer them. Pairing is
    /// positional — a result belongs to the most recent unanswered call — which is
    /// the same assumption both provider APIs enforce on the wire.
    pub fn observe_turn(&mut self, msgs: &[Value]) {
        let mut pending: Vec<(String, String)> = Vec::new();
        for m in msgs {
            let calls = calls_in(std::slice::from_ref(m));
            if !calls.is_empty() {
                pending.extend(calls);
                continue;
            }
            // A result, in either shape.
            let result = if m["role"] == "tool" {
                m["content"].as_str().map(str::to_string)
            } else if m["role"] == "user" {
                m["content"].as_array().and_then(|bs| {
                    bs.iter()
                        .find(|b| b["type"] == "tool_result")
                        .and_then(|b| b["content"].as_str().map(str::to_string))
                })
            } else {
                None
            };
            if let Some(r) = result {
                if !pending.is_empty() {
                    let (name, args) = pending.remove(0);
                    self.observe(&name, &args, &r);
                    self.step_done();
                }
            }
            if !self.open_still() {
                return;
            }
        }
    }

    /// Close one step of the window.
    pub fn step_done(&mut self) {
        self.steps_left = self.steps_left.saturating_sub(1);
    }

    /// A one-line summary for a trace line.
    pub fn summary(&self) -> String {
        format!(
            "compaction: dropped {} turn(s)/{} chars ({}), {} call(s) lost; next {} step(s): {} refetch, {} blocked",
            self.boundary.turns_dropped,
            self.boundary.chars_dropped,
            if self.boundary.summarized { "summarized" } else { "FIFO, no summary" },
            self.lost.len(),
            self.steps_seen,
            self.refetches,
            self.blocked
        )
    }

    /// The frame to append to the run log.
    pub fn frame(&self) -> Value {
        json!({
            // `type`, not `kind`: `runlog::append` reads this field and silently
            // drops a frame without it. A counter whose record is discarded on the
            // way to disk is worse than no counter, because it looks like one.
            "type": "compaction",
            "turns_dropped": self.boundary.turns_dropped,
            "chars_dropped": self.boundary.chars_dropped,
            "summarized": self.boundary.summarized,
            "calls_lost": self.lost.len(),
            "watched_steps": self.steps_seen,
            "refetches": self.refetches,
            "blocked": self.blocked,
        })
    }

    pub fn refetches(&self) -> usize {
        self.refetches
    }

    pub fn blocked(&self) -> usize {
        self.blocked
    }
}
