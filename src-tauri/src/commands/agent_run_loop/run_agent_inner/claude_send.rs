use super::*;

/// One step to Claude's native API. `None` when the user stopped the turn.
#[allow(clippy::too_many_arguments)]
pub(super) async fn claude_step(model: &str, settings: &crate::core::settings::Settings, sys_prompt: &crate::core::llm::SystemPrompt, body: &serde_json::Value, messages: &[serde_json::Value], key: &str, cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>, state: &AppState, sid: &str, persona: &str, api_model: &str) -> Result<Option<crate::core::llm::LlmReply>, String> {
    // The effort slider, cashed in. `thinking_field` returns None when the
    // level is the fast end OR when max_tokens leaves no room for a thinking
    // block, so the field is simply absent rather than present-and-invalid.
    //
    // The reply parser below already walks `content` by block type and pushes
    // the whole array back as the assistant turn, so thinking blocks survive
    // the round trip untouched — which the API requires once thinking and tool
    // use are on together.
    // One client — `core::llm`. The per-model thinking/effort shape (§2.1),
    // the retry policy (§2.6), the cancel race (§2.5) and the usage accounting
    // (§2.3) all live there now, so this loop is about the TURN again: what to
    // send, and what to do with what comes back.
    //
    // Each of those four was a fix that had to be made in four places, and
    // three of them were made in fewer. That is what Phase 2 is for.
    let req = crate::core::llm::LlmRequest::new(model, settings.max_tokens)
        // Blocks, with the cache breakpoint after the stable half — see
        // `SystemPrompt`. The volatile half still travels; it simply sits
        // after the marker instead of in front of it.
        .system(sys_prompt.blocks())
        .tools(body["tools"].as_array().cloned().unwrap_or_default())
        .messages(messages.to_vec())
        .effort(settings.effort.clone());
    match crate::core::llm::call_typed(&req, key, cancel).await {
        Ok(r) => Ok(r),
        // Same ending as the branch above: a sentence, and a row after it.
        Err(crate::core::llm::CallError::Stalled(silent)) => {
            Err(crate::core::stall::end_turn(&state.data_dir, sid, persona, api_model, silent, false))
        }
        Err(crate::core::llm::CallError::Failed(e)) => Err(e),
    }
}
