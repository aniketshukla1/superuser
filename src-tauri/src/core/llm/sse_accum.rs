use super::*;

/// A streamed Messages reply, assembled into the blocks the non-streaming API returns.
///
/// Split from the socket so every event shape can be tested directly — the parsing is
/// where the sharp edges are: tool input arrives as fragments of JSON that only parse
/// once joined, a thinking block's signature arrives in its own delta and must survive
/// the round trip untouched, and a server tool's result arrives whole in its start.
#[derive(Default)]
pub(super) struct SseAccum {
    pub(super) blocks: Vec<Value>,
    /// Tool input, as JSON text still arriving, by block index.
    pub(super) partial: std::collections::HashMap<usize, String>,
    pub(super) stop_reason: String,
    /// `message_start`'s usage, overlaid by each `message_delta`'s.
    pub(super) usage: serde_json::Map<String, Value>,
    pub(super) started: bool,
    pub(super) done: bool,
    pub(super) error: Option<String>,
}

impl SseAccum {
    pub(super) fn push_line(&mut self, line: &str) {
        // `event:` lines repeat what `data`'s own `type` says, and blank lines separate
        // events; only the data carries anything.
        let Some(data) = line.trim().strip_prefix("data:") else { return };
        let Ok(v) = serde_json::from_str::<Value>(data.trim()) else { return };
        let index = v["index"].as_u64().unwrap_or(0) as usize;
        match v["type"].as_str().unwrap_or("") {
            "message_start" => {
                self.started = true;
                if let Some(u) = v["message"]["usage"].as_object() {
                    self.usage.extend(u.clone());
                }
            }
            "content_block_start" => {
                while self.blocks.len() <= index {
                    self.blocks.push(json!({}));
                }
                self.blocks[index] = v["content_block"].clone();
            }
            "content_block_delta" => {
                let Some(block) = self.blocks.get_mut(index) else { return };
                let d = &v["delta"];
                let append = |block: &mut Value, key: &str, more: &str| {
                    let joined = format!("{}{more}", block[key].as_str().unwrap_or(""));
                    block[key] = Value::String(joined);
                };
                match d["type"].as_str().unwrap_or("") {
                    "text_delta" => append(block, "text", d["text"].as_str().unwrap_or("")),
                    "thinking_delta" => append(block, "thinking", d["thinking"].as_str().unwrap_or("")),
                    "signature_delta" => append(block, "signature", d["signature"].as_str().unwrap_or("")),
                    "input_json_delta" => {
                        self.partial.entry(index).or_default().push_str(d["partial_json"].as_str().unwrap_or(""));
                    }
                    "citations_delta" => {
                        if !block["citations"].is_array() {
                            block["citations"] = json!([]);
                        }
                        if let Some(list) = block["citations"].as_array_mut() {
                            list.push(d["citation"].clone());
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                if let (Some(text), Some(block)) = (self.partial.remove(&index), self.blocks.get_mut(index)) {
                    // No fragments at all is an empty object, which is what the
                    // non-streaming reply says for a tool called with no arguments.
                    block["input"] = if text.trim().is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
                    };
                }
            }
            "message_delta" => {
                if let Some(r) = v["delta"]["stop_reason"].as_str() {
                    self.stop_reason = r.to_string();
                }
                if let Some(u) = v["usage"].as_object() {
                    self.usage.extend(u.clone());
                }
            }
            "message_stop" => self.done = true,
            "error" => {
                self.error = Some(v["error"]["message"].as_str().unwrap_or("the stream reported an error").to_string());
            }
            _ => {} // `ping`, and anything newer than this client
        }
    }

    pub(super) fn finish(mut self, model: &str) -> LlmReply {
        // A block whose stop never arrived still carries whatever input it had.
        for (i, text) in std::mem::take(&mut self.partial) {
            if let Some(block) = self.blocks.get_mut(i) {
                block["input"] = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
            }
        }
        LlmReply {
            content: self.blocks,
            stop_reason: self.stop_reason,
            usage: usage_from(&json!({ "usage": Value::Object(self.usage) }), model),
        }
    }
}
