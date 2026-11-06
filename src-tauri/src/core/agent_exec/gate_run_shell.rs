use super::*;

/// Consult the auto-approve reviewer, if the user turned it on.
///
/// Called only on a decision the gate already marked `Ask` — never on a `Deny`.
/// That ordering is the invariant: the reviewer may only upgrade *ask the person* to
/// *proceed*, and nothing else. A model that can talk its way past a refusal is not a
/// reviewer, it is a bypass.
///
/// **Deliberately, a reviewer `deny` does not block the call** and leave the
/// user to override it from the event. Here a deny still
/// shows the card — annotated with the reviewer's objection — because the human
/// was going to see that card anyway without a reviewer, so showing it costs
/// nothing in safety and never leaves the user stuck behind a judge that got it
/// wrong. The reviewer's power is to REMOVE a card, never to remove a choice.
pub(super) async fn consult_reviewer(ctx: &ToolCtx<'_>, tool: &str, args: &Value) -> Reviewed {
    use crate::core::reviewer::{self, Verdict};

    if !ctx.settings.auto_approve || ctx.reviewer.is_paused() {
        return Reviewed::Human(None);
    }
    let req = reviewer::Request {
        user_said: ctx.prompt.clone(),
        tool: tool.to_string(),
        args: args.clone(),
        workspace: ctx.root.to_string_lossy().to_string(),
    };
    let verdict = reviewer::review(&ctx.settings, &req).await;
    let tripped = ctx.reviewer.record(&verdict);
    match verdict {
        Verdict::Allow { reason } => {
            crate::core::audit::log(&ctx.data_dir, "approve", &ctx.session_id, tool, &format!("auto-approved: {reason}"), true);
            Reviewed::Allow
        }
        // The judge abstained, which is it working correctly rather than being
        // obstructive. Straight to the human, with nothing extra on the card.
        Verdict::Unsure { .. } => Reviewed::Human(None),
        Verdict::Deny { reason } => {
            crate::core::audit::log(&ctx.data_dir, "deny", &ctx.session_id, tool, &format!("reviewer: {reason}"), true);
            Reviewed::Human(Some(if tripped {
                format!("The safety reviewer objected: {reason}\n\n{}", reviewer::PAUSED_NOTICE)
            } else {
                format!("The safety reviewer objected: {reason}")
            }))
        }
    }
}

/// The approval flow for `run_shell` — everything that happens before a command
/// is allowed to reach a shell.
///
/// Returns `Some(refusal)` to stop the call, or `None` to let it run.
///
/// This is the join the permission engine was missing. `inbox::decide` has said
/// it gates "writes, sends and shell" since it landed, and until now `send_message`
/// was its only caller: a shell command went straight to the executor whatever the
/// persona's mode said, and whether or not the run had been tainted by content
/// from off the machine. The GUI has meanwhile had the card ready the whole time —
/// `ApprovalCard` special-cases `run_shell` and offers "Always allow this command"
/// — waiting for a `permission_required` frame nothing emitted.
///
/// Order matters here:
///
/// 1. **The floor first.** A catastrophic command never becomes a card. Asking
///    "shall I `rm -rf /`?" invites a mis-click on a question that has no good
///    answer; `dispatch_local` refuses it with the reason instead.
/// 2. **Then what the user already blessed** — a session grant, checked through
///    [`cmdgate::auto_allowable`] so a blessed prefix still cannot carry a second
///    command in on its back.
/// 3. **Then the mode**, via [`inbox::decide`], which is also where taint costs a
///    run its shortcuts.
pub(super) async fn gate_run_shell(ctx: &ToolCtx<'_>, args: &Value) -> Option<String> {
    use crate::core::cmdgate::{self, grants};
    use crate::core::inbox::{decide, Ask, Decision, Inbox};

    let cmd = args["command"].as_str().unwrap_or("").trim().to_string();
    // An empty or catastrophic command is `dispatch_local`'s to answer — it owns
    // both messages, and both paths through it.
    if cmd.is_empty() || cmdgate::hard_deny(&cmd).is_some() {
        return None;
    }
    // What may run without asking: what the user blessed in this session, plus
    // what a TRUSTED project declares. An untrusted project contributes nothing,
    // so cloning a repo never grants it anything — see `core::workspace_trust`.
    let mut blessed = grants::commands(&ctx.session_id);
    blessed.extend(crate::core::workspace_trust::allowances(&ctx.data_dir, &ctx.root));
    if grants::tool_allowed(&ctx.session_id, "run_shell") || cmdgate::auto_allowable(&cmd, &blessed) {
        return None;
    }

    match decide("run_shell", &cmd, crate::core::agent::RiskClass::Exec, &ctx.mode, &ctx.standing, ctx.is_tainted()) {
        Decision::Allow => None,
        Decision::Deny => Some(format!(
            "Not run — this assistant's mode ({}) doesn't run commands. Say what the command \
             would have told you and let the user run it if they want it.",
            ctx.mode
        )),
        Decision::Ask => {
            let summary = format!("Run: {}", cmd.chars().take(140).collect::<String>());
            // A read-only command carries no decision, so it should not carry a
            // card. Checked inside the Ask branch on purpose: a mode that never
            // acts (`plan`, `discuss`) has already returned Deny above and is not
            // quietly reopened here. Taint does not disqualify it either — the
            // classifier admits no network client and no interpreter, so there is
            // nothing for an injected instruction to reach out with.
            if ctx.settings.auto_allow_read_only_commands && cmdgate::readonly::is_read_only(&cmd) {
                return None;
            }
            // Auto-approve, if the user turned it on. Consulted HERE — after the
            // floor and after the mode — so it can only ever skip the card, never
            // a refusal.
            let note = match consult_reviewer(ctx, "run_shell", args).await {
                Reviewed::Allow => return None,
                Reviewed::Human(note) => note,
            };
            if ctx.unattended {
                let ask = Inbox::load(&ctx.data_dir).park(Ask::new(
                    &ctx.session_id,
                    ctx.task_id.clone(),
                    "run_shell",
                    &cmd,
                    &summary,
                    args.clone(),
                ));
                return Some(format!(
                    "Parked for approval (inbox id {}). The command was NOT run — the user reviews it in the Inbox.",
                    ask.id
                ));
            }
            let payload = serde_json::json!({
                "name": "run_shell",
                "arguments": args.clone(),
                "reason": note.clone().unwrap_or_else(|| if ctx.is_tainted() {
                    "This run has read content from outside this machine, so commands are confirmed one at a time.".to_string()
                } else {
                    "Commands are confirmed before they run.".to_string()
                }),
                "category": "shell",
            });
            match prompt_user(ctx, "permission", &summary, payload, args).await {
                // "deny" is the only answer that stops it. Everything else is a
                // yes of some width — the same reading the Inbox already uses
                // (`api_server`'s resolution handling), so one dialect covers both.
                Ok(d) if d.trim() == "deny" => Some(format!(
                    "Denied by the user — the command was NOT run:\n\n{cmd}\n\nDon't retry it or reshape it \
                     into something that does the same thing. Ask what they'd rather you did."
                )),
                Ok(d) => {
                    match d.trim() {
                        "always_tool" => grants::allow_tool(&ctx.session_id, "run_shell"),
                        other if other.starts_with("always") => grants::allow_command(&ctx.session_id, &cmd),
                        _ => {}
                    }
                    crate::core::audit::log(&ctx.data_dir, "approve", &ctx.session_id, &cmd, &summary, true);
                    None
                }
                // No answer came back (timeout, stop, nothing attached). The
                // message already says what to do about it, and it is never
                // "assume yes".
                Err(e) => Some(e),
            }
        }
    }
}
