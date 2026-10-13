use super::*;

impl LlmReply {
    /// Concatenated text blocks — what a caller with no tools wants.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
    /// The `tool_use` blocks, in the order the model emitted them.
    pub fn tool_uses(&self) -> Vec<Value> {
        self.content.iter().filter(|b| b["type"] == "tool_use").cloned().collect()
    }
}

/// Usage, in the shape `core::pricing` and the stats card read.
///
/// The cache counters are separate fields on purpose: Anthropic bills them separately,
/// `input_tokens` EXCLUDES them, and folding them together would make Phase 3
/// unmeasurable — a cached step and an uncached one reporting the same number.
pub fn usage_from(v: &Value, model: &str) -> Option<TokenUsage> {
    let u = v.get("usage")?;
    let n = |k: &str| u.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let usage = TokenUsage {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_read: n("cache_read_input_tokens"),
        cache_write: n("cache_creation_input_tokens"),
        model: Some(model.to_string()),
    };
    (!usage.is_empty()).then_some(usage)
}

/// Why a call produced no reply.
#[derive(Debug, Clone, PartialEq)]
pub enum CallError {
    /// Nothing arrived for this long — see `core::stall`.
    Stalled(std::time::Duration),
    Failed(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Stalled(d) => write!(f, "Claude sent nothing for {}", crate::core::stall::in_words(*d)),
            CallError::Failed(e) => write!(f, "{e}"),
        }
    }
}

pub(super) const ANTHROPIC_MESSAGES: &str = "https://api.anthropic.com/v1/messages";

/// Send it. `Ok(None)` means the caller's stop flag went up — not an error, and not a
/// reply either.
///
/// Every policy this app has learned lives here rather than at the call site: retry on
/// the failures that clear on their own, abandon the request when Stop is pressed, and
/// never retry a refusal.
pub async fn call(req: &LlmRequest, key: &str, cancel: &Arc<AtomicBool>) -> Result<Option<LlmReply>, String> {
    call_typed(req, key, cancel).await.map_err(|e| e.to_string())
}

/// [`call`], with a stall told apart from every other failure — the assistant loop
/// ends a stalled turn with its own sentence and a trail row, which it can only do if
/// it can tell.
pub async fn call_typed(req: &LlmRequest, key: &str, cancel: &Arc<AtomicBool>) -> Result<Option<LlmReply>, CallError> {
    call_at(ANTHROPIC_MESSAGES, req, key, cancel, crate::core::stall::limit(false)).await
}

/// The call, against any address, with any silence limit — which is what lets a test
/// point it at a server on loopback that goes quiet on purpose.
///
/// # Streamed
///
/// The request asks for `stream: true` and the reply is assembled here into exactly
/// the shape the non-streaming API returned, so no caller changes. It streams for one
/// reason: a non-streaming request sends nothing at all until the whole answer is
/// generated, which makes "still thinking" and "hung" indistinguishable. A stream
/// sends blocks as they form and a `ping` in between, so silence means something —
/// and Anthropic's own guidance is to stream anything long for the same reason.
pub(super) async fn call_at(
    url: &str,
    req: &LlmRequest,
    key: &str,
    cancel: &Arc<AtomicBool>,
    limit: Option<std::time::Duration>,
) -> Result<Option<LlmReply>, CallError> {
    use crate::core::stall::{self, Quiet};
    use futures_util::StreamExt;

    let http = crate::core::egress::client("model");
    let mut body = req.body();
    body["stream"] = json!(true);
    crate::trace!(
        "llm",
        "request model={} tools={} thinking={:?} effort={:?}",
        req.model,
        req.tools.len(),
        body.get("thinking"),
        body.get("output_config")
    );

    let mut attempt = 1u32;
    let resp = loop {
        if cancel.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let mut builder = http
            .post(url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");
        if let Some(b) = &req.beta {
            builder = builder.header("anthropic-beta", b.clone());
        }
        // Race the call against the stop flag. Checked only BETWEEN steps, cancellation
        // did nothing until a long inference returned — measured at ~25s once, which
        // reads as a dead button. The same wait times the silence before the headers.
        let sent = match stall::within(builder.json(&body).send(), limit, cancel).await {
            Ok(r) => r.map_err(|e| CallError::Failed(format!("Claude request failed: {e}")))?,
            Err(Quiet::Stopped) => return Ok(None),
            Err(Quiet::Stalled(d)) => return Err(CallError::Stalled(d)),
        };
        let status = sent.status().as_u16();
        match retry::decide(status, attempt, retry::retry_after_secs(sent.headers())) {
            Next::Done => break sent,
            Next::RetryIn(d) => {
                crate::trace!("llm", "{status} — retrying in {d:?} (attempt {attempt}/{})", retry::MAX_ATTEMPTS);
                tokio::time::sleep(d).await;
                attempt += 1;
            }
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(CallError::Failed(format!("Claude API error {status}: {text}")));
    }

    let mut acc = SseAccum::default();
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    while !acc.done {
        let next = match stall::within(stream.next(), limit, cancel).await {
            Ok(n) => n,
            Err(Quiet::Stopped) => return Ok(None),
            Err(Quiet::Stalled(d)) => return Err(CallError::Stalled(d)),
        };
        let Some(chunk) = next else { break };
        buf.extend_from_slice(&chunk.map_err(|e| CallError::Failed(format!("Claude stream failed: {e}")))?);
        while let Some(nl) = buf.iter().position(|b| *b == b'\n') {
            let line = String::from_utf8_lossy(&buf[..nl]).into_owned();
            buf.drain(..=nl);
            acc.push_line(&line);
        }
        if let Some(e) = &acc.error {
            return Err(CallError::Failed(format!("Claude API error: {e}")));
        }
    }
    if !buf.is_empty() {
        acc.push_line(&String::from_utf8_lossy(&buf));
    }
    if let Some(e) = acc.error {
        return Err(CallError::Failed(format!("Claude API error: {e}")));
    }
    if !acc.started {
        return Err(CallError::Failed("Claude's reply ended before it began".into()));
    }
    let reply = acc.finish(&req.model);
    crate::trace!(
        "llm",
        "reply blocks={} stop={} usage={:?}",
        reply.content.len(),
        reply.stop_reason,
        reply.usage
    );
    Ok(Some(reply))
}
