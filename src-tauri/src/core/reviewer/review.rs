use super::*;

/// Render the one action under review. Kept small and fixed-shape so the judge
/// sees the same structure every time.
pub(super) fn action_block(req: &Request) -> String {
    // Arguments are JSON the model proposed, not content read from anywhere — see
    // invariant 4. Truncated so a large `write_file` body cannot crowd out the
    // instructions; the judge is deciding about the ACTION, not proofreading it.
    let args = serde_json::to_string(&req.args).unwrap_or_else(|_| "{}".into());
    let args: String = args.chars().take(1200).collect();
    format!(
        "Working folder: {}\n\nThe user asked:\n{}\n\nProposed action:\n{} {}",
        if req.workspace.is_empty() { "(none)" } else { &req.workspace },
        if req.user_said.trim().is_empty() { "(nothing this turn)" } else { req.user_said.trim() },
        req.tool,
        args
    )
}

/// Ask the judge. Never returns an error: every failure is [`Verdict::Unsure`].
pub async fn review(settings: &Settings, req: &Request) -> Verdict {
    let msgs = vec![
        ChatMessage { role: "system".into(), content: INSTRUCTIONS.into(), images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: action_block(req), images: vec![], docs: vec![] },
    ];
    // `BackgroundCapable`: stays on the local model (a judge is not worth billing
    // per action, and it must work with the network off) but keeps the full model
    // rather than the small one — the answer has to hit an exact format, and a
    // model that reasons correctly and then fails to emit it produces `Unsure`
    // every time, which is a feature that silently never works.
    let call = model_router::complete(settings, &msgs, model_router::Workload::BackgroundCapable);
    match tokio::time::timeout(TIMEOUT, call).await {
        Ok(Ok((reply, _))) if !reply.trim().is_empty() => parse(&reply),
        Ok(Ok(_)) => Verdict::unsure("the reviewer returned nothing"),
        Ok(Err(e)) => Verdict::unsure(format!("the reviewer could not be reached ({e})")),
        Err(_) => Verdict::unsure("the reviewer took too long to answer"),
    }
}
