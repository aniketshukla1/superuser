use super::*;

/// Where a repair may be sent. Constructed only on the Ollama branch of the loop, so
/// a wire that does not speak `format` cannot reach this code by accident.
#[derive(Debug, Clone)]
pub struct Repair {
    /// The model the turn is already running on — repairing on a different one would
    /// be answering a question nobody asked.
    pub model: String,
    /// The `/api/chat` endpoint, resolved the same way the turn's own request was.
    pub url: String,
}

/// A repair is one extra round trip inside a step the user is already waiting on. It
/// is worth a few seconds and never worth a hang: past this, the error text the model
/// would have got anyway is strictly the better answer.
pub(super) const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

/// What the model is told. Deliberately short: the tool, what the user asked for, the
/// arguments that failed and why. Everything else it needs is in the schema, which is
/// being enforced rather than described.
pub(super) fn instruction(tool: &str, description: &str, ask: &str, bad: &Value, why: &str) -> String {
    // A call whose arguments never parsed arrives here as `{}` — `read_arguments`
    // substitutes it so the call keeps a well-formed shape for the UI, and the text the
    // model actually emitted is gone by then. Quoting `{}` back at it as "what you
    // sent" would be a small lie about a call that in fact sent something truncated, so
    // that line is omitted instead. The ask and the schema carry the repair on their
    // own — measured: a constrained request with no prior-arguments context at all
    // still produced the right object on both small models.
    let sent = match bad.as_object() {
        Some(o) if o.is_empty() => String::new(),
        _ => format!(
            "The arguments that were sent: {}\n\n",
            serde_json::to_string(bad).unwrap_or_else(|_| "{}".into())
        ),
    };
    format!(
        "A call to the tool `{tool}` ({description}) could not be used.\n\n\
         What the user asked for: {ask}\n\n\
         {sent}\
         Why it was rejected: {why}\n\n\
         Reply with the corrected arguments for `{tool}` and nothing else. Keep every \
         value that was already right; change only what the rejection names. Do not \
         invent a path, a filename or a value the user did not give you."
    )
}

/// The wire body for a repair.
///
/// Split out so the shape can be asserted without a socket — the same split
/// `llm::Request::body` and `retry::decide` make, and for the same reason: the sharp
/// edges are in what gets sent, and nothing that needs a network to exercise gets
/// exercised.
///
/// Three fields carry the whole design. `format` is the schema being enforced. There
/// is no `tools` key, because its presence would suppress `format`. And `think` is
/// pinned false — see the module docs for what it costs otherwise.
pub fn body(model: &str, schema: &Value, tool: &str, description: &str, ask: &str, bad: &Value, why: &str) -> Value {
    let mut body = json!({
        "model": model,
        "stream": false,
        "think": false,
        "format": schema,
        // Nothing here is a matter of taste. The model is re-emitting a decision it
        // has already made, and sampling around it only invents new ways to be wrong.
        "options": { "temperature": 0 },
        "messages": [{
            "role": "user",
            "content": instruction(tool, description, ask, bad, why),
        }],
    });
    // The turn's own request holds the model in memory; a repair that let it fall out
    // would charge the next step a reload it never asked for.
    crate::core::ollama::stamp_keep_alive(&mut body, model);
    body
}

/// Whether what came back is usable, given the schema that was sent.
///
/// Two checks, both of which have to pass. It must parse — a server that ignored
/// `format` entirely returns prose, and prose is not arguments. And it must satisfy
/// `check_args`, which is the same validator the call would face at dispatch. Trusting
/// the grammar instead would mean the one place a repair could silently return
/// something the loop then refuses.
pub fn accept(schema: &Value, content: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(content.trim()).ok()?;
    crate::core::loopshape::check_args(schema, &parsed).ok()?;
    Some(parsed)
}

/// Ask the model for arguments it cannot get wrong. `None` means the caller should do
/// exactly what it did before this existed.
///
/// One attempt, never two. A repair that failed produced either a server that does not
/// speak `format` or a model that cannot satisfy the schema at temperature 0, and
/// neither is fixed by asking twice — while the user waits through both.
pub async fn attempt(
    http: &crate::core::egress::Client,
    repair: &Repair,
    spec: &crate::core::agent::ToolSpec,
    ask: &str,
    bad: &Value,
    why: &str,
) -> Option<Value> {
    let body = body(&repair.model, &spec.schema, &spec.name, &spec.description, ask, bad, why);
    let began = std::time::Instant::now();
    let resp = http.post(&repair.url).timeout(TIMEOUT).json(&body).send().await.ok()?;
    if !resp.status().is_success() {
        crate::trace!("turn", "arg repair {}: HTTP {}", spec.name, resp.status());
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    let content = v["message"]["content"].as_str().unwrap_or_default();
    let out = accept(&spec.schema, content);
    crate::trace!(
        "turn",
        "arg repair {} {:.1}s -> {}",
        spec.name,
        began.elapsed().as_secs_f64(),
        if out.is_some() { "accepted" } else { "rejected" }
    );
    out
}

/// A tool call the model *printed* instead of making, recovered into a real one.
///
/// The third way a call reaches us as text, and the only one with no recovery path
/// until now. `read_arguments` catches unparseable arguments and `check_args`
/// catches the wrong shape; both assume a call was actually made. This is the case
/// where the model emitted `{"name":"read_file","parameters":{…}}` as its answer and
/// the loop, seeing no `tool_calls`, treated the machinery as the reply.
///
/// It is not rare and it is not evenly spread. `universal_check` has a clause for it
/// because it kept happening on `llama3.2`, which accounts for four of the brain
/// group's twelve failures and six to eight per arm of the tool-selection ablation —
/// every one of them scored as a wrong answer when the model had in fact chosen
/// correctly and only failed to say so on the wire.
///
/// **Conservative on purpose.** The whole trimmed message must be one JSON object
/// with a name and an argument bag, optionally inside a ```json fence, and the name
/// must be a tool this turn actually has. Prose that merely contains JSON is left
/// alone: a model explaining a call it is about to make is not making it, and
/// executing that would be inventing an action the user never got.
pub fn salvage_printed_call(text: &str, available: &[String]) -> Option<Value> {
    salvage_printed_call_with(text, available, &[])
}

/// The same salvage, and additionally willing to find the call **inside prose** —
/// for read-only tools only.
///
/// The strict version never fired on the sweep it was built for. `llama3.2` does not
/// print a bare call object; it narrates one: *"Running `list_dir` to find the actual
/// file path, I get: {"name": "list_dir", …}"* — and then answers from a result it
/// imagined. The strict rule refuses that on purpose, because a model describing a
/// call has not made it, and running a narrated `send_message` would invent an action
/// the user never saw proposed.
///
/// That objection is about side effects, and a read has none. Running a narrated
/// `list_dir` does nothing the user could object to and replaces an imagined result
/// with a real one — which is strictly better than the model reasoning from a listing
/// it made up. So prose-embedded calls are recovered for `RiskClass::Read` tools and
/// refused for everything else, and `read_only` is how the caller says which is which.
pub fn salvage_printed_call_with(text: &str, available: &[String], read_only: &[String]) -> Option<Value> {
    if let Some(v) = strict(text, available) {
        return Some(v);
    }
    if read_only.is_empty() {
        return None;
    }
    for candidate in json_objects(text) {
        if let Some(v) = strict(&candidate, read_only) {
            return Some(v);
        }
    }
    None
}
