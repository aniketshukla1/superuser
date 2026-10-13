use super::super::*;

impl StreamAccum {
    /// Feed one line. Returns whatever that line carried: answer text, reasoning,
    /// or neither.
    pub fn push_line(&mut self, line: &str, openai: bool) -> StreamDelta {
        let none = StreamDelta::default();
        let line = line.trim();
        if line.is_empty() {
            return none; // SSE keep-alive / record separator
        }
        let payload = if openai {
            let Some(rest) = line.strip_prefix("data:").map(str::trim) else { return none };
            if rest == "[DONE]" {
                self.done = true;
                return none;
            }
            rest
        } else {
            line
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) else { return none };

        // Usage, wherever this provider puts it.
        //
        // OpenAI sends it in a trailing chunk whose `choices` is EMPTY — which is why this
        // runs before the delta read below rather than inside it — and only when the request
        // asked for it with stream_options.include_usage. Ollama puts its counts on the
        // final `done` object under different names. Anthropic is not streamed here and is
        // handled at its call site.
        //
        // Read additively, not assigned: a provider that reports input early and output late
        // would otherwise have the first number overwritten by a chunk that omits it.
        let (in_key, out_key) =
            if openai { ("prompt_tokens", "completion_tokens") } else { ("prompt_eval_count", "eval_count") };
        let slot = if openai { &v["usage"] } else { &v };
        let (i, o) = (slot[in_key].as_u64().unwrap_or(0), slot[out_key].as_u64().unwrap_or(0));
        if i > 0 || o > 0 {
            let acc = self.usage.get_or_insert_with(Default::default);
            acc.input = acc.input.max(i);
            acc.output = acc.output.max(o);
        }

        let (text, thinking, deltas) = if openai {
            let d = &v["choices"][0]["delta"];
            (
                d["content"].as_str().unwrap_or(""),
                // Both spellings are in the wild across OpenAI-compatible providers.
                d["reasoning_content"].as_str().or_else(|| d["reasoning"].as_str()).unwrap_or(""),
                d["tool_calls"].as_array().cloned(),
            )
        } else {
            let m = &v["message"];
            (
                m["content"].as_str().unwrap_or(""),
                m["thinking"].as_str().unwrap_or(""),
                m["tool_calls"].as_array().cloned(),
            )
        };
        for d in deltas.unwrap_or_default() {
            // Ollama sends complete calls; only OpenAI's arrive in fragments.
            if openai {
                self.merge_tool_call_delta(&d);
            } else {
                self.calls.push(d);
            }
        }
        if !openai && v["done"].as_bool().unwrap_or(false) {
            self.done = true;
        }
        let mut out = StreamDelta::default();
        if !thinking.is_empty() {
            self.thinking.push_str(thinking);
            out.thinking = Some(thinking.to_string());
        }
        if !text.is_empty() {
            self.content.push_str(text);
            out.text = Some(text.to_string());
        }
        out
    }

    /// Merge one OpenAI streaming `tool_calls` delta into the accumulating list.
    ///
    /// OpenAI splits a single call across chunks — the name arrives once, then `arguments`
    /// comes in fragments that only parse once concatenated — with `index` saying which
    /// call each fragment belongs to.
    pub(in crate::commands) fn merge_tool_call_delta(&mut self, delta: &serde_json::Value) {
        use serde_json::json;
        let idx = delta["index"].as_u64().unwrap_or(0) as usize;
        while self.calls.len() <= idx {
            self.calls.push(json!({ "id": "", "type": "function", "function": { "name": "", "arguments": "" } }));
        }
        let slot = &mut self.calls[idx];
        if let Some(id) = delta["id"].as_str().filter(|s| !s.is_empty()) {
            slot["id"] = json!(id);
        }
        if let Some(name) = delta["function"]["name"].as_str().filter(|s| !s.is_empty()) {
            slot["function"]["name"] = json!(name);
        }
        if let Some(frag) = delta["function"]["arguments"].as_str() {
            let joined = format!("{}{frag}", slot["function"]["arguments"].as_str().unwrap_or(""));
            slot["function"]["arguments"] = json!(joined);
        }
    }

    /// The finished message, in the shape the non-streaming response produced.
    pub fn finish(self) -> serde_json::Value {
        let mut message = serde_json::json!({ "role": "assistant", "content": self.content });
        if !self.calls.is_empty() {
            message["tool_calls"] = serde_json::json!(self.calls);
        }
        message
    }
}

/// Read a streamed chat completion, handing each text fragment to `on_text` as it lands,
/// and return the finished message in the shape the non-streaming call returned.
///
/// The local path used to request `stream: false`, so nothing reached the UI until the
/// whole turn was computed — an 8-second answer was 8 seconds of blank spinner, which is
/// most of what "too slow" actually meant. The wall-clock time is the same; what changes is
/// that the first token shows up in a second instead of at the end.
///
/// Handles both wire formats: OpenAI-style SSE (`data: {...}` lines, `[DONE]` terminator,
/// deltas under `choices[0].delta`) and Ollama's newline-delimited JSON (a whole `message`
/// per line, `done: true` at the end).
pub(crate) async fn stream_chat_message<F: Fn(String), G: Fn(String)>(
    resp: reqwest::Response,
    openai: bool,
    cancel: &std::sync::atomic::AtomicBool,
    // Silence tolerated between chunks; `None` waits forever. See `core::stall`.
    limit: Option<std::time::Duration>,
    on_text: F,
    on_thinking: G,
) -> Result<StreamedMessage, String> {
    use futures_util::StreamExt;
    use serde_json::json;
    use std::sync::atomic::Ordering;

    let mut acc = StreamAccum::default();
    let mut body = resp.bytes_stream();
    // Bytes, not chars: a chunk boundary can land inside a multi-byte character, so lines
    // are split on the raw bytes and decoded only once complete.
    let mut buf: Vec<u8> = Vec::new();

    while !acc.done {
        // Checked per chunk so Stop still lands in well under a second mid-stream.
        if cancel.load(Ordering::SeqCst) {
            return Ok(StreamedMessage { message: json!({}), stopped: true, usage: acc.usage, stalled: None });
        }
        // Waiting on the provider is the only place a turn can hang, so it is the only
        // wait the watchdog times — and it also checks Stop while nothing is arriving,
        // which a bare `next().await` never did.
        let next = match crate::core::stall::within(body.next(), limit, cancel).await {
            Ok(n) => n,
            Err(crate::core::stall::Quiet::Stopped) => {
                return Ok(StreamedMessage { message: json!({}), stopped: true, usage: acc.usage, stalled: None })
            }
            Err(crate::core::stall::Quiet::Stalled(silent)) => {
                return Ok(StreamedMessage { message: acc.finish(), stopped: false, usage: None, stalled: Some(silent) })
            }
        };
        let Some(chunk) = next else { break };
        buf.extend_from_slice(&chunk.map_err(|e| format!("model stream failed: {e}"))?);

        while let Some(nl) = buf.iter().position(|b| *b == b'\n') {
            let line = String::from_utf8_lossy(&buf[..nl]).into_owned();
            buf.drain(..=nl);
            let d = acc.push_line(&line, openai);
            if let Some(t) = d.thinking {
                on_thinking(t);
            }
            if let Some(text) = d.text {
                on_text(text);
            }
            if acc.done {
                break;
            }
        }
    }
    // A final line without a trailing newline still carries content.
    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf).into_owned();
        let d = acc.push_line(&line, openai);
        if let Some(t) = d.thinking {
            on_thinking(t);
        }
        if let Some(text) = d.text {
            on_text(text);
        }
    }

    let usage = acc.usage.clone();
    Ok(StreamedMessage { message: acc.finish(), stopped: false, usage, stalled: None })
}
