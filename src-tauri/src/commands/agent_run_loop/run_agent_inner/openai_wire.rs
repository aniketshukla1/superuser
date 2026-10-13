use super::*;

/// Where an OpenAI-compatible (or Ollama) turn goes and what it offers.
pub(super) struct OpenAiWire {
    pub(super) openai: bool,
    pub(super) local_model: bool,
    pub(super) silence: Option<std::time::Duration>,
    pub(super) url: String,
    pub(super) bearer: Option<String>,
    pub(super) repair: Option<crate::core::argrepair::Repair>,
    pub(super) tools: Vec<serde_json::Value>,
    pub(super) can_think: bool,
}

pub(super) fn openai_wire(t: &Turn<'_>) -> Result<OpenAiWire, String> {
    let Turn { prov, model, settings, specs, mcp, profile, .. } = t;
    let model_can_think = t.model_can_think;
    use crate::core::providers::ProviderKind;
    let openai = matches!(prov.map(|p| p.kind), Some(ProviderKind::OpenAiCompatible));
    // How long this model may say nothing before the turn is ended. A model on this
    // machine gets far longer: its silences are loading and reading, not faults.
    let local_model = crate::core::providers::runs_locally(&model);
    let silence = crate::core::stall::limit(local_model);
    let (url, bearer) = if openai {
        let p = prov.unwrap();
        // The override store decides the address and the registry default is only
        // the fallback. That is the whole reason a provider can be pointed at a
        // gateway, a self-hosted server, or a proxy on loopback without the model
        // id, the tool schemas, or this loop changing at all.
        let base = crate::core::providers::base_url(p.id);
        crate::core::providers::endpoint(&base, crate::core::providers::key_for(p.id), p.name)?
    } else {
        (format!("{}/api/chat", settings.ollama_url.trim_end_matches('/')), None)
    };
    // A repair goes to the same model at the same endpoint as the turn itself, and
    // only where that endpoint speaks `format`. Ollama does. The OpenAI-compatible
    // spelling — `response_format: {type: "json_schema"}` — is the same idea in
    // different words, and is deliberately not sent: it is unmeasured here, and a
    // gateway that ignores an unknown field would turn a repair into a wasted round
    // trip inside a step the user is waiting on. See `core::argrepair`.
    let repair = (!openai).then(|| crate::core::argrepair::Repair {
        model: model.clone(),
        url: url.clone(),
    });
    let mut tools = tools_ollama(&specs);
    tools.extend(mcp.ollama_tools.iter().cloned());
    // Sorted again with the connectors' tools folded in, so the whole array is
    // ordered rather than "ours in order, then theirs in whatever order the
    // servers answered in". `gather_mcp` walks a fixed connector list and then
    // the registry, which is stable in practice — but it is stable by accident,
    // and the cost of it drifting is a full prefill on every step of every turn.
    // A server that drops out between turns changes the *set* and invalidates the
    // prefix regardless; this only promises that an unchanged set keeps its order.
    tools.sort_by(|a, b| {
        a["function"]["name"].as_str().unwrap_or("").cmp(b["function"]["name"].as_str().unwrap_or(""))
    });
    // Cloud providers stream reasoning without being asked; the flag is Ollama-only.
    // Which vocabulary this model speaks is a profile field, not a vendor guess.
    let can_think = model_can_think
        && profile.thinking == Some(crate::core::model_profile::ThinkingStyle::Flag);
    Ok(OpenAiWire { openai, local_model, silence, url, bearer, repair, tools, can_think })
}

/// The tool calls in a step's message, including one the model printed instead of making.
pub(super) fn salvage_printed(msg: &mut serde_json::Value, specs: &[crate::core::agent::ToolSpec]) -> Vec<serde_json::Value> {
    let mut calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
    // The call the model printed instead of making. Seen constantly on
    // `llama3.2`: the loop finds no `tool_calls`, takes the machinery as the
    // answer, and a turn where the right tool was chosen is scored as a
    // wrong answer. Recovered only when the whole message is one call object
    // naming a tool this turn actually has — see `argrepair`.
    if calls.is_empty() {
        let printed = msg["content"].as_str().unwrap_or("");
        let available: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
        // Read-only tools may be recovered out of prose; nothing else may.
        let read_only: Vec<String> = specs
            .iter()
            .filter(|s| s.risk == crate::core::agent::RiskClass::Read)
            .map(|s| s.name.clone())
            .collect();
        if let Some(recovered) =
            crate::core::argrepair::salvage_printed_call_with(printed, &available, &read_only)
        {
            crate::trace!("turn", "recovered a tool call the model printed as text");
            calls = recovered.as_array().cloned().unwrap_or_default();
            msg["tool_calls"] = recovered;
            msg["content"] = serde_json::Value::String(String::new());
        }
    }
    calls
}
