use super::*;

/// What we ship knowing, for a model id in the registry.
///
/// Anthropic's own answers are better than this in every field — see
/// [`parse_anthropic_model`] — so this is the fallback for a run with no key, not the
/// primary source. It deliberately leaves `context_window` unset: the honest number
/// comes from `/v1/models`, and a hardcoded 200k that quietly goes stale is exactly
/// the folklore H1 is about.
pub fn declared(model: &str) -> Option<ModelProfile> {
    let prov = providers::resolve(model)?;
    let name = providers::model_name(model);
    // Only ids we actually ship support for. An unrecognised id under a known
    // provider is still a guess and must say so.
    if !prov.models.iter().any(|m| *m == name) {
        return None;
    }
    let mut p = assumed(model);
    p.source = ProfileSource::Declared;
    p.tier = cloud_tier(model);
    match prov.kind {
        ProviderKind::Anthropic => {
            p.tools = ToolSupport::Parallel;
            // From `core::effort`'s per-model tables, which are maintained against the
            // docs and tested. One table, read from two places, rather than two tables.
            p.thinking = if crate::core::effort::accepts_adaptive(name) {
                Some(ThinkingStyle::Adaptive)
            } else if crate::core::effort::accepts_budget(name) {
                Some(ThinkingStyle::Budget)
            } else {
                None
            };
            p.vision = true;
            p.pdf = true;
            p.caching = Some(CacheStyle::Explicit);
        }
        ProviderKind::OpenAiCompatible => {
            p.tools = ToolSupport::Parallel;
            p.thinking = Some(ThinkingStyle::EffortName);
            p.vision = true;
        }
        ProviderKind::CodexCli | ProviderKind::ClaudeCli => {
            // The CLI runs its own agent with its own tools; none of our loop's
            // questions apply to it, and answering them would invite a caller to act
            // on the answer.
            p.tools = ToolSupport::None;
            p.thinking = None;
        }
        ProviderKind::Ollama => {}
    }
    Some(p)
}

// ---------------------------------------------------------------------------
// The handshake probe — what makes a custom endpoint first class
// ---------------------------------------------------------------------------

/// The two trivial tools a probe offers. Two, not one, because offering one can never
/// tell `Sequential` from `Parallel`.
pub(super) fn probe_tools() -> Value {
    let tool = |name: &str| {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": format!("Report the {name} status. Takes no arguments."),
                "parameters": { "type": "object", "properties": {} }
            }
        })
    };
    serde_json::json!([tool("ping"), tool("pong")])
}

/// What a probe response says about the server, independent of transport.
pub fn read_probe(body: &Value) -> (ToolSupport, bool) {
    let calls = body["choices"][0]["message"]["tool_calls"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let tools = match calls {
        0 => ToolSupport::None,
        1 => ToolSupport::Sequential,
        _ => ToolSupport::Parallel,
    };
    // `usage` present *and* carrying a non-zero count. A server that echoes an empty
    // usage object reports nothing, and recording it as reporting would put a zero
    // where a blank belongs.
    let reports_usage = body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) > 0
        || body["usage"]["input_tokens"].as_u64().unwrap_or(0) > 0;
    (tools, reports_usage)
}

/// What a handshake measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probed {
    pub tools: ToolSupport,
    pub reports_usage: bool,
    /// The server accepted `reasoning_effort`. Measured rather than assumed, because
    /// getting this wrong costs the whole turn: a model with no reasoning parameter
    /// answers 400 and every tool call the turn had already run is discarded.
    /// Observed live on `qwen2.5:7b-instruct` — HARNESS-BASELINE records it.
    pub accepts_effort: bool,
}

/// The probe's own request body.
///
/// `local` adds `keep_alive`, for a reason that is not about the probe: on a local
/// runner this request is very often what *loads* the model, and without the field
/// it would load it under Ollama's five-minute default and then hand the real turn
/// a model already counting down. The probe pays the load; the turn should inherit
/// a warm one.
///
/// Decided from the ADDRESS rather than from the model id, because the id reaching
/// this function is the bare wire name — the `provider:` prefix has already been
/// stripped — and `runs_locally` reads a bare name it does not recognise as local.
/// True for `qwen2.5:7b-instruct` and wrong for a Groq or Together model, which
/// would be sent an Ollama field for no reason. The URL is unambiguous.
pub(super) fn probe_body(model: &str, effort: Option<&str>, local: bool) -> Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": "Call both the ping and pong tools now. Do not reply with prose."
        }],
        "tools": probe_tools(),
        "tool_choice": "auto",
        // Enough room for two tool calls and nothing else. A 1-token cap — which the
        // plan suggested — cannot produce a tool call at all, so it would measure
        // "no tool support" on every server that has it.
        "max_tokens": 128,
        "stream": false,
        "temperature": 0
    });
    if let Some(e) = effort {
        body["reasoning_effort"] = Value::String(e.to_string());
    }
    if local {
        body["keep_alive"] = Value::String(crate::core::ollama::KEEP_ALIVE.to_string());
    }
    body
}

/// Ask an OpenAI-compatible endpoint what it can do, by making it do it.
///
/// Two trivial tools and a prompt that asks for both — one tool could never tell
/// `Sequential` from `Parallel`. The first attempt carries `reasoning_effort`; if the
/// server refuses it and the same request without it succeeds, that difference *is*
/// the measurement. Two round trips at worst, once per `(base_url, model)`.
///
/// Used only where the shipped table has nothing. Probing to replace a guess is worth
/// a round trip; probing to confirm something we ship is not.
pub(super) async fn probe_openai(
    http: &crate::core::egress::Client,
    url: &str,
    bearer: Option<&str>,
    model: &str,
) -> Option<Probed> {
    let send = |body: Value| async move {
        let mut req = http.post(url).json(&body);
        if let Some(k) = bearer {
            req = req.header("authorization", format!("Bearer {k}"));
        }
        let resp = req
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .ok()
            .filter(|r| r.status().is_success())?;
        resp.json::<Value>().await.ok()
    };

    let local = providers::is_loopback(url);
    if let Some(v) = send(probe_body(model, Some("low"), local)).await {
        let (tools, reports_usage) = read_probe(&v);
        return Some(Probed { tools, reports_usage, accepts_effort: true });
    }
    // It refused with the reasoning field. Whether that field was the reason is what
    // the second attempt settles — and if this one fails too, we learned nothing and
    // say so by returning `None` rather than recording a capability from a failure.
    let v = send(probe_body(model, None, local)).await?;
    let (tools, reports_usage) = read_probe(&v);
    Some(Probed { tools, reports_usage, accepts_effort: false })
}

// ---------------------------------------------------------------------------
// The cache
// ---------------------------------------------------------------------------

/// How long a discovered or probed profile is trusted before it is asked again.
///
/// Short enough that pulling a model or repointing a gateway is picked up inside a
/// session; long enough that a turn never waits on a probe twice.
pub(super) const TTL: Duration = Duration::from_secs(15 * 60);

pub(super) type Cache = Mutex<HashMap<String, (Instant, ModelProfile)>>;
