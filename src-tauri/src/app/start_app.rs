use super::*;

pub(super) fn start_automation_scheduler(handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    fn now_secs() -> f64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }
    core::supervise::keep_running("the automation scheduler", handle, move |handle| async move {
        tokio::time::sleep(std::time::Duration::from_secs(25)).await;
        loop {
            let due = {
                let state = handle.state::<AppState>();
                let mut store = core::automation::AutomationStore::load(&state.data_dir);
                // Before anything fires: write off runs the app died in the middle of, and
                // step past windows that are too late to be worth running. A laptop shut
                // for a week should wake up to today's work, not to a week of it.
                let reaped = store.reap_abandoned(now_secs());
                let skipped = store.skip_missed(now_secs());
                if reaped > 0 || !skipped.is_empty() {
                    crate::trace!("automation", "reaped {reaped} abandoned run(s), skipped {} missed window(s)", skipped.len());
                }
                store.due(now_secs())
            };
            for task in due {
                // Record the run.
                let run = {
                    let state = handle.state::<AppState>();
                    match core::automation::AutomationStore::load(&state.data_dir).start_run(&task.id, "schedule") {
                        Ok(r) => r,
                        Err(_) => continue,
                    }
                };
                // Open the run's conversation BEFORE it runs, and write the
                // instruction into it as the opening message.
                //
                // A scheduled run's session used to be a working folder and nothing
                // else — its directory held the files the agent wrote and no
                // `session.json` at all, because the transcript is persisted by the
                // socket and a scheduled run has no socket. Meanwhile the Scheduled
                // page offers "open this run's conversation" on every row. Twenty
                // runs, twenty invitations, and every one of them landed on an empty
                // chat: the conversation was never recorded to open.
                {
                    let state = handle.state::<AppState>();
                    let store = core::sessions::SessionStore::new(&state.data_dir);
                    if store.get(&run.session_id).is_none() {
                        let mut s = core::sessions::Session::with_id(
                            &run.session_id,
                            &task.agent,
                            &task.workspace,
                            task.model.as_deref().unwrap_or(""),
                        );
                        s.title = task.title.clone();
                        store.save(&s);
                    }
                    store.append(&run.session_id, core::sessions::Message::new("user", task.instructions.clone()));
                }
                // Execute the assistant.
                crate::trace!(
                    "automation",
                    "run id={} title={:?} agent={} model={:?} trigger={}",
                    task.id,
                    task.title,
                    task.agent,
                    task.model,
                    run.trigger
                );
                let result = {
                    let state = handle.state::<AppState>();
                    // Framed, because without it the instruction reads as a question ABOUT
                    // the automation rather than as the work. The bug this prevents: "every
                    // run, append a line to the Notion page" was read as a scheduling
                    // question, answered by listing the routines, and recorded as a success.
                    let firing = core::automation::framed(&task.instructions, false);
                    commands::run_agent(&handle, &state, task.agent.clone(), firing, Some(run.session_id.clone()), task.model.clone(), None).await
                };
                // …and the answer, so the conversation is a conversation. A failure is
                // recorded too: a run that died deserves a transcript saying so more
                // than a successful one does.
                {
                    let state = handle.state::<AppState>();
                    let store = core::sessions::SessionStore::new(&state.data_dir);
                    let msg = match &result {
                        Ok(o) => core::sessions::Message::new("assistant", o.answer.clone()).with_usage(o.usage.clone()),
                        Err(e) => core::sessions::Message::new("assistant", format!("That run failed: {e}")),
                    };
                    store.append(&run.session_id, msg);
                }
                // Finalize the run + roll the schedule forward.
                {
                    let state = handle.state::<AppState>();
                    let mut store = core::automation::AutomationStore::load(&state.data_dir);
                    match &result {
                        Ok(out) => { let _ = store.finish_run(&task.id, &run.run_id, "ok", Some(out.answer.clone()), out.artifacts.clone(), None); }
                        Err(e) => { let _ = store.finish_run(&task.id, &run.run_id, "error", None, vec![], Some(e.clone())); }
                    }
                    let _ = store.advance(&task.id, now_secs());
                    // The fatigue rule: one message the first time it breaks after working,
                    // silence through the middle, and the tenth switches it off and says so.
                    // An automation that fails every morning must not write every morning.
                    if let Err(e) = &result {
                        if let Some(word) = store.fatigue(&task.id, e) {
                            let sessions = core::sessions::SessionStore::new(&state.data_dir);
                            let where_to = task.origin_session.clone().unwrap_or_else(|| run.session_id.clone());
                            sessions.append(&where_to, core::sessions::Message::new("assistant", word));
                        }
                    }
                }
                // Write the answer back into the conversation this task came from,
                // so the user can follow up on it there instead of digging through
                // run records. Only on success: a failure belongs in the run log,
                // not injected into a chat as if it were an answer.
                if let (Some(origin), Ok(out)) = (task.origin_session.as_deref(), &result) {
                    let state = handle.state::<AppState>();
                    let store = core::sessions::SessionStore::new(&state.data_dir);
                    store.inject_scheduled(origin, &task.id, &task.title, &out.answer, SCHEDULED_PAIRS_PER_SESSION);
                }

                // Notify.
                if task.notify_on_completion {
                    let msg = match &result {
                        Ok(_) => format!("✓ {} finished", task.title),
                        Err(_) => format!("⚠️ {} needs attention", task.title),
                    };
                    if let Some(mascot) = handle.get_webview_window("mascot") {
                        let _ = mascot.emit("mascot-message", serde_json::json!({ "message": msg, "type": "message" }).to_string());
                    }
                    if let Some(main) = handle.get_webview_window("main") {
                        let _ = main.emit("automation-run", serde_json::json!({ "task": task.id }));
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
}

/// Slack as an entry point: mention the bot in a channel and an assistant session
/// runs here, with the answer posted back in-thread. Polls every ~20s (Web API,
/// no websocket); each message is handled exactly once via a stored watermark.
pub(super) fn start_slack_watcher(handle: tauri::AppHandle) {
    use tauri::Manager;
    // Not in a scratch instance: it holds the real Slack token and none of the real state, so
    // with no record of which mentions were answered it would reply again under every recent
    // mention in the person's workspace. See `core::instance::is_scratch`.
    if core::instance::is_scratch() {
        log_line(&handle, "scratch instance: Slack watcher not started");
        return;
    }
    core::supervise::keep_running("the Slack watcher", handle, move |handle| async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        loop {
            let connected = integrations::slack::bot_token().is_some();
            if connected {
                let data_dir = { handle.state::<AppState>().data_dir.clone() };
                match integrations::slack::poll_mentions(&data_dir).await {
                    Ok(mentions) => {
                        for m in mentions {
                            log_line(&handle, &format!("slack: working on a mention in #{}", m.channel_name));
                            let session = format!("slack-{}-{}", m.channel, m.ts.replace('.', "-"));
                            // Tell the assistant where the request came from, so it can
                            // answer for that channel and address the person who asked.
                            let ask = format!(
                                "You were asked this in Slack #{} by <@{}>. Answer for them, concisely enough to read in a thread.\n\n{}",
                                m.channel_name, m.user, m.text
                            );
                            let result = {
                                let state = handle.state::<AppState>();
                                let persona = crate::core::personas::DEFAULT_PERSONA_ID.to_string();
                                commands::run_agent(&handle, &state, persona, ask, Some(session), None, None).await
                            };
                            let reply = match result {
                                Ok(out) => {
                                    let mut r = out.answer;
                                    if !out.artifacts.is_empty() {
                                        r.push_str(&format!("\n\n_Files produced: {}_", out.artifacts.join(", ")));
                                    }
                                    r
                                }
                                Err(e) => format!("I couldn't finish that: {e}"),
                            };
                            if let Err(e) = integrations::slack::post_reply(&m.channel, &m.thread_ts, &reply).await {
                                log_line(&handle, &format!("slack: reply failed: {e}"));
                            }
                        }
                    }
                    Err(e) => log_line(&handle, &format!("slack: poll failed: {e}")),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(if connected { 20 } else { 120 })).await;
        }
    });
}
