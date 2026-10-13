use super::*;
use crate::core::agentbrowser::computer;

/// `browser_request_help`: hand the browser to the user and wait until they give it back,
/// decline, or time out.
pub(super) async fn request_help(ctx: &ToolCtx<'_>, c: &computer::Computer, cancel: &std::sync::atomic::AtomicBool, reason: String) -> String {
    if reason.is_empty() {
        return "error: say what the user needs to do, in one sentence.".into();
    }
    let page = computer::current_url(&ctx.persona).unwrap_or_default();
    // Not parked in the Inbox: approving an Inbox item replays the call, and a
    // replayed request for hands has nobody's hands to wait for. The browser shows
    // it needs you instead, and a sign-in done there is kept for the next run.
    if ctx.unattended {
        c.request_help(&reason, &ctx.session_id);
        return "Nobody is watching this run, so nobody can take your browser now. The user will see that it needs \
                them. Don't try to do it yourself — finish by saying what you needed and why."
            .into();
    }
    let Some(tx) = &ctx.events else {
        return format!(
            "error: nothing is attached to this run, so nobody can take your browser. Say in your answer what the user \
             needs to do{}, and don't try to do it yourself.",
            if page.is_empty() { String::new() } else { format!(" on {page}") }
        );
    };
    let (released_before, _) = c.counters();
    let declined_before = c.declines();
    c.request_help(&reason, &ctx.session_id);
    let _ = tx.send(serde_json::json!({"type": "computer_help", "persona": ctx.persona, "reason": reason, "url": page}));
    let deadline = std::time::Instant::now() + HANDS_TIMEOUT;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if cancel.load(Ordering::SeqCst) {
            return "The user stopped this turn before handing the browser back. Stop here.".into();
        }
        if c.counters().0 > released_before {
            return "The user took control, finished, and handed your browser back. Take a new snapshot before acting \
                    — the page may have changed while they had it."
                .into();
        }
        if c.declines() > declined_before {
            return "The user said not now — they won't take your browser for this. Don't try to do it yourself: \
                    say what you needed it for and stop there."
                .into();
        }
        let view = c.control();
        if !view.requested && view.holder == crate::core::agentbrowser::control::Holder::Assistant {
            return "Nobody took your browser in time. Say what you still need rather than trying to do it yourself.".into();
        }
        if std::time::Instant::now() >= deadline {
            return "The user still hasn't handed your browser back after 30 minutes. End your turn by saying what you \
                    were waiting for."
                .into();
        }
    }
}

/// `browser_request_secret`: have the user type a value straight into the page, unseen.
pub(super) async fn request_secret(ctx: &ToolCtx<'_>, c: &computer::Computer, cancel: &std::sync::atomic::AtomicBool, label: String, target_ref: String, id: u64) -> String {
    if ctx.unattended {
        return "Not done — nobody is watching this run, so nobody can type that value. Finish by saying what you needed.".into();
    }
    let Some(tx) = &ctx.events else {
        return "error: nothing is attached to this run, so nobody can type that value. Say in your answer what you needed; \
                don't ask for it any other way."
            .into();
    };
    let (_, supplied_before) = c.counters();
    if let Err(e) = c.request_secret(&label, &target_ref, id, &ctx.session_id) {
        return browser_failed(e);
    }
    let wanted = c.control().secret_wanted.unwrap_or_else(|| label.clone());
    let _ = tx.send(serde_json::json!({"type": "computer_secret", "persona": ctx.persona, "label": wanted}));
    let deadline = std::time::Instant::now() + HANDS_TIMEOUT;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if cancel.load(Ordering::SeqCst) {
            return "The user stopped this turn. Stop here.".into();
        }
        if c.counters().1 > supplied_before {
            return format!(
                "The user entered {wanted} into the field. It went straight into the page; you weren't told what it is. \
                 Submitting the form is up to you."
            );
        }
        if c.control().secret_wanted.is_none() || std::time::Instant::now() >= deadline {
            return format!("Nobody entered {wanted}. Don't ask for it any other way.");
        }
    }
}
