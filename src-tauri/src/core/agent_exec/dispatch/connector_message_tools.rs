use super::*;

pub(super) async fn connector_message_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    match name {
        "connector_changes" => {
            let id = args["connector"].as_str().unwrap_or("").trim().to_lowercase();
            if id.is_empty() {
                return "error: name a connector".into();
            }
            let peek = args["peek"].as_bool().unwrap_or(false);
            let ledger = crate::core::ledger::Ledger::new(&ctx.memory.vault_path);
            match ledger.changes_since_read(&id, !peek) {
                Err(e) => format!("No change history for {id}: {e}"),
                Ok(c) if c.is_empty() => format!("{} ({} item(s) in total, none of them new).", c.summary(), c.total),
                Ok(c) => {
                    // Titles, not bodies: this answers "what should I look at",
                    // and the agent can read any of them with connector_search.
                    let list = |label: &str, files: &[String]| -> String {
                        if files.is_empty() {
                            return String::new();
                        }
                        let names: Vec<String> = files
                            .iter()
                            .take(20)
                            .map(|f| {
                                ctx.memory
                                    .parse_file(&ctx.memory.vault_path.join(f))
                                    .map(|m| m.title)
                                    .unwrap_or_else(|| f.clone())
                            })
                            .collect();
                        let more = files.len().saturating_sub(names.len());
                        format!(
                            "\n\n### {label}\n{}{}",
                            names.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n"),
                            if more > 0 { format!("\n- …and {more} more") } else { String::new() }
                        )
                    };
                    format!(
                        "{}{}{}{}",
                        c.summary(),
                        list("Added", &c.added),
                        list("Changed", &c.modified),
                        // Removed items have no file left to read a title from, so
                        // the filename is all there is — say so rather than pretend.
                        if c.removed.is_empty() { String::new() } else { format!("\n\n### Removed\n{}", c.removed.iter().take(20).map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n")) }
                    )
                }
            }
        }
        "connector_sync" => {
            let id = args["connector"].as_str().unwrap_or("").trim().to_lowercase();
            if id.is_empty() {
                return "error: name a connector to sync".into();
            }
            let engine = crate::core::memory::MemoryEngine::new(ctx.memory.vault_path.clone());
            // The gateway recorded the sync before it ran, and records a failure after.
            let res = crate::integrations::sync_one(&id, &engine).await;
            match res {
                Ok(out) => format!("Synced {id}: {} new item(s).{}", out.created, if out.titles.is_empty() { String::new() } else { format!(" Newest: {}", out.titles.join("; ")) }),
                Err(e) => format!("Couldn't sync {id}: {e}"),
            }
        }
        "send_message" => {
            use crate::core::inbox::{decide, Ask, Decision, Inbox};
            let target = args["target"].as_str().unwrap_or("").trim().to_string();
            let text = args["text"].as_str().unwrap_or("");
            if target.is_empty() || text.trim().is_empty() {
                return "error: send_message needs a target (channel:address) and text".into();
            }
            // The session's Access panel is the authority on how far a source may
            // reach HERE, and it is consulted before the approval machinery: a
            // source the user marked read-only should never reach the Inbox as a
            // pending ask, because there is no answer the user could give that
            // ought to send it. Only a grant actually made can narrow this — an
            // unset one leaves the send exactly as wide as it was before scopes.
            let (platform, _) = crate::core::senders::parse_target(&target);
            if crate::core::sessions::SessionStore::new(&ctx.data_dir).may_write(&ctx.session_id, &platform)
                == Some(false)
            {
                // "Not sent" is what the gateway reads as declined, and records.
                return format!(
                    "Not sent — {platform} is read-only in this session (Access ▸ Sources). Nothing left the machine. The draft:\n\n{text}"
                );
            }
            let summary = format!("Send to {target}: {}", text.chars().take(140).collect::<String>());
            match decide("send_message", &target, crate::core::agent::RiskClass::External, &ctx.mode, &ctx.standing, ctx.is_tainted()) {
                Decision::Deny => format!("Not sent — this assistant's mode ({}) doesn't allow sending. Report the draft instead:\n\n{text}", ctx.mode),
                Decision::Allow => {
                    // Recorded by the gateway before it left, by target and length — never
                    // by its words, which are the one thing the trail must not hold. A reply
                    // the email bot drafted carries the message it answers, so it threads.
                    let res = crate::core::senders::send_threaded(&target, text, args["in_reply_to"].as_str(), args["references"].as_str()).await;
                    match res {
                        Ok(msg) => msg,
                        Err(e) => format!("Send failed: {e}"),
                    }
                }
                Decision::Ask => {
                    // The same reviewer, on the more dangerous of the two paths: a
                    // send leaves the machine and cannot be recalled. It is gated
                    // identically — the judge may skip the card, never the mode.
                    if matches!(consult_reviewer(ctx, "send_message", args).await, Reviewed::Allow) {
                        let res = crate::core::senders::send_threaded(&target, text, args["in_reply_to"].as_str(), args["references"].as_str()).await;
                        return match res {
                            Ok(msg) => msg,
                            Err(e) => format!("Send failed: {e}"),
                        };
                    }
                    // Unattended runs never act on their own — park the ask for the user.
                    let ask = Inbox::load(&ctx.data_dir).park(Ask::new(&ctx.session_id, ctx.task_id.clone(), "send_message", &target, &summary, args.clone()));
                    format!(
                        "Parked for approval (inbox id {}). The message was NOT sent — the user reviews it in the Inbox.{}",
                        ask.id,
                        if ctx.unattended { "" } else { " Tell the user it's waiting for their approval." }
                    )
                }
            }
        }
        // `computer_use`, `spawn_subagent` and `load_skill` need app-level context
        // (the Tauri handle, the persona registry, the skill store), so the caller
        // resolves them before falling through here — see `commands::run_tool`.
        // They used to be answered here with "isn't available yet", which had been
        // untrue since both were wired up: a caller that forgot to intercept would
        // be told the feature doesn't exist instead of that it routed wrong.
        "computer_use" | "spawn_subagent" | "load_skill" => {
            format!("internal error: '{name}' must be dispatched by the agent loop, not the tool executor.")
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
