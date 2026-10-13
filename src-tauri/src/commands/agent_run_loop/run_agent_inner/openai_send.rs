use super::*;
use serde_json::json;

/// One step's request body.
#[allow(clippy::too_many_arguments)]
pub(super) fn step_body(w: &OpenAiWire, api_model: &str, max_steps: usize, profile: &crate::core::model_profile::ModelProfile, settings: &crate::core::settings::Settings, model: &str, messages: &Vec<serde_json::Value>, spent: usize) -> serde_json::Value {
    let (openai, can_think, tools) = (w.openai, w.can_think, &w.tools);
    // Streamed: the answer paints as it's generated instead of appearing all at
    // once when the step finishes. Same total time, a fraction of the waiting.
    let mut body = json!({"model":api_model,"stream":true,"messages":messages});
    // No tools on the final step, which is what makes the notice above true
    // rather than a hint. Omitted rather than sent empty: `"tools": []` is
    // rejected outright by some OpenAI-compatible servers.
    if !crate::core::loopshape::is_final_step(spent, max_steps) {
        body["tools"] = json!(tools);
    }
    if openai {
        // Without this an OpenAI-compatible stream reports no usage at all, which is
        // why the app has never had a token count. Part of the OpenAI spec and
        // ignored by servers that predate it; a server that REJECTS it is handled at
        // the send site, which retries once without. A probe that measured the
        // server answering with no usage saves that round trip entirely.
        if profile.reports_usage {
            body["stream_options"] = json!({ "include_usage": true });
        }
        // Same slider, the other vendor's vocabulary: a name, not a budget, and
        // nothing above "high" exists there. Absent for the fast end.
        //
        // Gated on the profile because sending it to a server without it is not
        // harmless: a `custom:` endpoint fronting a model with no reasoning
        // parameter answers 400 and the whole turn is lost — measured on
        // `qwen2.5:7b-instruct`, and recorded in HARNESS-BASELINE.
        if profile.thinking == Some(crate::core::model_profile::ThinkingStyle::EffortName) {
            if let Some(e) = crate::core::effort::reasoning_effort(&settings.effort) {
                body["reasoning_effort"] = json!(e);
            }
        }
    }
    if !openai {
        // Ollama defaults to the model's own (often small) context window unless
        // told otherwise, and silently drops the front of the conversation when
        // it overflows — the assistant losing its own instructions mid-task. The
        // chat path already sent these; this loop never did, which also made the
        // budget above a fiction. `num_ctx` covers input + output together.
        body["options"] = json!({ "num_ctx": profile.ollama_num_ctx(&settings), "num_predict": settings.max_tokens });
        // Show the working, when the model can produce it. Gated on the
        // advertised capability: Ollama rejects `think` outright for a model
        // without it, which would fail the turn rather than just omit a nicety.
        if can_think {
            body["think"] = json!(true);
        }
    }
    // Hold the model in memory between turns — see `ollama::KEEP_ALIVE`.
    //
    // Deliberately OUTSIDE the `!openai` gate above. Everything in that block is
    // native-Ollama vocabulary, and `keep_alive` reads like more of the same, but
    // the shim honours it too and the configuration that needs it most is exactly
    // the one the gate excluded: `custom:` pointed at loopback, which is how the
    // owner's own machine runs and how most local users reach anything that isn't
    // stock Ollama. Gated on the address instead, inside `stamp_keep_alive`.
    crate::core::ollama::stamp_keep_alive(&mut body, &model);
    body
}

/// Sends one step, racing Stop and silence. `None` when the user stopped the turn.
#[allow(clippy::too_many_arguments)]
pub(super) async fn send_step(w: &OpenAiWire, http: &crate::core::egress::Client, state: &AppState, cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>, sid: &str, persona: &str, api_model: &str, mut body: serde_json::Value) -> Result<Option<reqwest::Response>, String> {
    let OpenAiWire { url, bearer, silence, local_model, .. } = w;
    let (silence, local_model) = (*silence, *local_model);
    let mut req = http.post(url).json(&body);
    if let Some(k) = &bearer { req = req.header("authorization", format!("Bearer {k}")); }
    // Race the model call against this session's stop flag. Cancellation used
    // to be checked only BETWEEN steps, so a Stop pressed during a long
    // inference did nothing until that call returned — measured at ~25s once,
    // which reads as a dead button. Abandoning the request makes Stop land in
    // well under a second.
    //
    // The same wait is timed for silence (`core::stall`): a local server sends
    // its headers with the first token, so the time until they arrive is the
    // model saying nothing, and it counts against the same limit.
    let resp = match crate::core::stall::within(req.send(), silence, &cancel).await {
        Ok(r) => r.map_err(|e| format!("model request failed: {e}"))?,
        Err(crate::core::stall::Quiet::Stopped) => return Ok(None),
        Err(crate::core::stall::Quiet::Stalled(silent)) => {
            return Err(crate::core::stall::end_turn(&state.data_dir, &sid, &persona, &api_model, silent, local_model));
        }
    };
    // A server that rejects an OPTIONAL field must not cost you the turn. Token
    // counts and a reasoning hint are both niceties; talking to the model is the
    // product. One retry without them, only on a 4xx and only when we actually
    // sent one — a 5xx is the server being unwell and asking again the same way
    // is just noise.
    //
    // `reasoning_effort` joined `stream_options` here because it was sent
    // ungated to every OpenAI-compatible endpoint, while the `think` field a few
    // lines above is capability-checked with a comment saying precisely why:
    // "Ollama rejects `think` outright for a model without it, which would fail
    // the turn rather than just omit a nicety." The same was true here and the
    // same care was not taken. On a local qwen2.5 served over an
    // OpenAI-compatible endpoint, every turn at any effort above the left stop
    // died with `"qwen2.5:7b-instruct" does not support thinking` — the whole
    // turn lost, the answer never written, and a raw JSON 400 in the transcript.
    // Observed repeatedly on the owner's own default model, 2026-09-01.
    //
    // A retry rather than a capability table: an arbitrary OpenAI-compatible
    // endpoint cannot be asked what it supports, and guessing from the model id
    // is how the wrong guess becomes permanent.
    // Dropped ONE AT A TIME, most-likely culprit first, so a server that only
    // objects to the reasoning hint still gets to report its token usage.
    // Removing both at once made every qwen2.5 turn succeed with `usage: None`,
    // which is a quieter version of the same problem HARNESS-PLAN Phase 0 was
    // written to end: work whose cost nobody records.
    let mut resp = resp;
    let mut stopped_on_retry = false;
    // `keep_alive` joins the list for the same reason the other two are on it: it
    // is a nicety, talking to the model is the product, and an endpoint that has
    // never heard of the field must not cost the user their turn. Dropped LAST —
    // it is only ever sent to a loopback address, so it is the least likely of
    // the three to be what a server objected to.
    for field in ["reasoning_effort", "stream_options", "keep_alive"] {
        if resp.status().is_success() || !resp.status().is_client_error() {
            break;
        }
        if body.get(field).is_none() {
            continue;
        }
        body.as_object_mut().map(|b| b.remove(field));
        let mut retry = http.post(url).json(&body);
        if let Some(k) = &bearer {
            retry = retry.header("authorization", format!("Bearer {k}"));
        }
        resp = match crate::core::stall::within(retry.send(), silence, &cancel).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("model request failed: {e}")),
            Err(crate::core::stall::Quiet::Stopped) => {
                stopped_on_retry = true;
                break;
            }
            Err(crate::core::stall::Quiet::Stalled(silent)) => {
                return Err(crate::core::stall::end_turn(&state.data_dir, &sid, &persona, &api_model, silent, local_model));
            }
        };
    }
    if stopped_on_retry {
        return Ok(None);
    }
    if !resp.status().is_success() {
        let st = resp.status();
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("model error {st}: {}", t.chars().take(200).collect::<String>()));
    }
    Ok(Some(resp))
}
