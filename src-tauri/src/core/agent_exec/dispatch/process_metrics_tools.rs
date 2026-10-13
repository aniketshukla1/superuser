use super::*;

pub(super) async fn process_metrics_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    match name {
        // Reading back what `context::relieve_pressure` cut out of this turn.
        // Here rather than in `dispatch_local` because the store is per session and
        // `dispatch_local` has no ctx — it cannot know which conversation is asking,
        // and answering with another session's output would be the leak the store's
        // injective directory naming exists to prevent.
        // Long-running processes — `core::procs`. The command gate lives in there, on the
        // command and on every line sent, so no path into a running process skips it.
        "process_start" => {
            let cwd = match args["cwd"].as_str().map(str::trim).filter(|c| !c.is_empty()) {
                Some(c) => ctx.root.join(c),
                None => ctx.root.clone(),
            };
            match crate::core::procs::start(&ctx.session_id, &cwd.to_string_lossy(), args["command"].as_str().unwrap_or("")) {
                Ok(h) => format!(
                    "started {h}. It is running in the background; call process_read with handle \"{h}\" to see its output, \
                     and process_stop when you are done with it."
                ),
                Err(e) => e,
            }
        }
        "process_read" => {
            let h = args["handle"].as_str().unwrap_or("");
            let wait = args["wait_seconds"].as_u64().unwrap_or(0).min(10);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait);
            let mut acc = String::new();
            loop {
                match crate::core::procs::read(&ctx.session_id, h) {
                    Err(e) => return e,
                    Ok((chunk, exit)) => {
                        acc.push_str(&chunk);
                        if exit.is_some() || !acc.trim().is_empty() || std::time::Instant::now() >= deadline {
                            let status = match exit {
                                Some(code) => format!("[exited with code {code}]"),
                                None => "[still running]".to_string(),
                            };
                            return if acc.trim().is_empty() {
                                format!("(no new output) {status}")
                            } else {
                                format!("{}\n{status}", acc.trim_end())
                            };
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        }
        "process_send" => {
            let newline = args["newline"].as_bool().unwrap_or(true);
            match crate::core::procs::send(&ctx.session_id, args["handle"].as_str().unwrap_or(""), args["input"].as_str().unwrap_or(""), newline) {
                Ok(()) => "sent. Call process_read to see how it responded.".into(),
                Err(e) => e,
            }
        }
        "process_stop" => match crate::core::procs::stop(&ctx.session_id, args["handle"].as_str().unwrap_or("")) {
            Ok(tail) if tail.trim().is_empty() => "stopped.".into(),
            Ok(tail) => format!("stopped. Its last output:\n{}", tail.trim_end()),
            Err(e) => e,
        },
        "expense_summary" => crate::core::expenses::summary_tool(&ctx.data_dir, &ctx.settings, args),
        "metrics_find" | "metrics_query" | "metrics_range" | "metrics_compare" | "metrics_alerts" | "perf_report" => {
            let conn = match crate::integrations::prometheus::Conn::load() {
                Ok(c) => c,
                Err(e) => return format!("error: {e}"),
            };
            match crate::core::perf::run(&conn, name, args, &ctx.settings.perf_signals, &ctx.root, chrono::Utc::now()).await {
                Ok(out) => {
                    // Label values and alert text are the monitored systems' words.
                    ctx.taint();
                    for f in &out.files {
                        ctx.note_seen(f);
                    }
                    crate::core::webguard::wrap_untrusted(&conn.url, &out.text)
                }
                Err(e) => format!("error: {e}"),
            }
        }
        "activity_since" => crate::core::chief_of_staff::tool(ctx.memory, &ctx.settings, args),
        "recall_output" => {
            let store = crate::core::observations::ObservationStore::for_session(
                &ctx.data_dir,
                &ctx.session_id,
            );
            let Some(id) = args["id"].as_str().filter(|s| !s.is_empty()) else {
                return "error: recall_output needs the `id` from a truncation notice.".into();
            };
            // A window if asked for, the whole thing otherwise. `from` without `len`
            // means "from here to the end", which is what someone reading a long
            // result actually wants on the second call.
            let from = args["from"].as_u64().map(|v| v as usize);
            let len = args["len"].as_u64().map(|v| v as usize);
            match (from, len) {
                (None, None) => match store.get(id) {
                    Some(text) => text,
                    None => format!(
                        "error: no truncated output is held under id {id:?} for this conversation. \
                         Handles come from a \"… [N characters truncated …] …\" notice in this \
                         same turn and do not outlive the session."
                    ),
                },
                (f, l) => {
                    let f = f.unwrap_or(0);
                    let l = l.unwrap_or(usize::MAX);
                    match store.slice(id, f, l) {
                        // Saying the total is what makes a second call possible: the
                        // model can see whether it has reached the end.
                        Some((text, total)) => format!(
                            "[characters {f}..{} of {total}]\n{text}",
                            (f.saturating_add(text.chars().count())).min(total)
                        ),
                        None => format!("error: no truncated output is held under id {id:?} for this conversation."),
                    }
                }
            }
        }
        "ask_user" => ask_user(ctx, args).await,
        "ask_form" => ask_form(ctx, args).await,
        "report_refusal" => {
            let request = args["request"].as_str().unwrap_or("").trim();
            let reason = args["reason"].as_str().unwrap_or("").trim();
            if request.is_empty() || reason.is_empty() {
                return "error: say what you were asked (`request`) and why you are declining (`reason`)".into();
            }
            let facts = ctx.gateway_facts();
            let mut e = crate::core::audit::Entry::event("assistant.declined");
            e.actor = facts.persona.clone();
            e.persona = facts.persona;
            e.session = facts.session;
            e.initiator = Some(facts.initiator);
            // The assistant's own paraphrase, bounded — never the user's message itself.
            e.target = request.chars().take(160).collect();
            e.detail = reason.chars().take(300).collect();
            crate::core::audit::append(&ctx.data_dir, e);
            "Recorded. Now tell the user plainly that you won't do it, and why — this note is not the answer.".into()
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
