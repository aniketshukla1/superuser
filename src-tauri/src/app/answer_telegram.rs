use super::*;

/// Two-way Telegram: answer messages sent to the bot, and keep the exchange in the
/// app as an ordinary conversation you can read and carry on from either end.
///
/// Off unless `telegram_inbound` is on AND at least one chat is allowlisted —
/// anyone can message a bot they find, and this one answers with an agent that has
/// file and shell access.
pub(super) fn start_telegram_watcher(handle: tauri::AppHandle) {
    use tauri::Manager;
    // Not in a scratch instance: its replies would come from a throwaway folder, through the
    // person's real bot. See `core::instance::is_scratch`.
    if core::instance::is_scratch() {
        log_line(&handle, "scratch instance: Telegram watcher not started");
        return;
    }
    core::supervise::keep_running("the Telegram watcher", handle, move |handle| async move {
        tokio::time::sleep(std::time::Duration::from_secs(35)).await;
        loop {
            let (settings, data_dir) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.data_dir.clone())
            };
            let on = settings.telegram_inbound
                && integrations::telegram::bot_token().is_some()
                && !integrations::telegram::allowed_chats(&settings).is_empty();

            if on {
                match integrations::telegram::poll(&data_dir, &settings).await {
                    Ok(messages) => {
                        for m in messages {
                            log_line(&handle, &format!("telegram: answering {}", m.chat_id));
                            if let Err(e) =
                                answer_telegram(&handle, &data_dir, &settings.ollama_model, &m.chat_id, &m.from, &m.text).await
                            {
                                log_line(&handle, &format!("telegram: reply failed: {e}"));
                            }
                        }
                    }
                    Err(e) => log_line(&handle, &format!("telegram: poll failed: {e}")),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(if on { 10 } else { 120 })).await;
        }
    });
}

/// Answer one inbound Telegram message: make the conversation real, run the agent
/// on it with the chat's own history, save the reply and send it back.
///
/// Pulled out of the poll loop because there is a second caller now. A message from
/// an unlisted chat is parked (`core::unrouted`), and when the user reads it and
/// picks "allow and deliver", that message has to be answered through exactly this
/// path — same session id, same history, same reply channel — or a delivered
/// message would be a different kind of conversation from a polled one, which is
/// the sort of difference nobody discovers until a thread reads wrong.
pub async fn answer_telegram(
    handle: &tauri::AppHandle,
    data_dir: &std::path::Path,
    model: &str,
    chat_id: &str,
    from: &str,
    text: &str,
) -> Result<(), String> {
    use tauri::Manager;
    let sid = integrations::telegram::session_id(chat_id);
    let store = core::sessions::SessionStore::new(data_dir);
    // Make the session real before the run, so the sidebar shows the conversation
    // while the answer is still coming.
    if store.get(&sid).is_none() {
        let mut s = core::sessions::Session::with_id(
            &sid,
            core::personas::DEFAULT_PERSONA_ID,
            &data_dir.to_string_lossy(),
            model,
        );
        s.origin = Some("telegram".into());
        s.title = format!("Telegram · {from}");
        store.save(&s);
    }
    store.append(&sid, core::sessions::Message::new("user", text.to_string()));

    // The history is the session's own, so a reply on the phone continues the same
    // conversation the app shows.
    let history: Vec<commands::HistMsg> = store
        .get(&sid)
        .map(|s| s.history_wired())
        .unwrap_or_default()
        .into_iter()
        .map(|(role, content, wire)| commands::HistMsg { role, content, wire })
        .collect();

    let reply = {
        let state = handle.state::<AppState>();
        let persona = core::personas::DEFAULT_PERSONA_ID.to_string();
        let ask = format!(
            "This came from {from} over Telegram, so keep the answer short enough to read on a phone.\n\n{text}"
        );
        match commands::run_agent(handle, &state, persona, ask, Some(sid.clone()), None, Some(history)).await {
            Ok(out) => out.answer,
            Err(e) => format!("I couldn't finish that: {e}"),
        }
    };
    store.append(&sid, core::sessions::Message::new("assistant", reply.clone()));
    let sent = core::senders::send(&format!("telegram:{chat_id}"), &reply).await;
    if let Some(main) = handle.get_webview_window("main") {
        use tauri::Emitter;
        let _ = main.emit("sessions-changed", serde_json::json!({ "session": sid }));
    }
    sent.map(|_| ())
}

/// The email bot (`core::email_bot`): new mail from the senders you allow gets a reply drafted
/// by the Email Assistant, and the draft waits in the Inbox until you approve it.
///
/// Off unless `email_inbound` is on, Email (IMAP) is connected, and someone is on the list —
/// anyone can write to you. Looks once a minute; reads only mail that arrived after it was
/// switched on.
pub(super) fn start_email_watcher(handle: tauri::AppHandle) {
    use tauri::Manager;
    // Not in a scratch instance: it would read the person's mailbox from a throwaway folder,
    // and — launched to share accounts — draft into their Inbox from it. See
    // `core::instance::is_scratch`.
    if core::instance::is_scratch() {
        log_line(&handle, "scratch instance: email bot not started");
        return;
    }
    core::supervise::keep_running("the email bot", handle, move |handle| async move {
        tokio::time::sleep(std::time::Duration::from_secs(45)).await;
        loop {
            let (settings, data_dir) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.data_dir.clone())
            };
            // One mailbox, two readers: the email bot drafting replies, and the Expense
            // Manager logging receipts (`core::expenses`). Either one switched on is a look.
            let list = core::email_bot::allowlist(&settings.email_allowed);
            let replies = settings.email_inbound && !list.is_empty();
            let on = core::secrets::has_integration_token("email") && (replies || settings.expense_intake);
            if on {
                email_look(&handle, &data_dir, &settings, &list).await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(if on { 60 } else { 120 })).await;
        }
    });
}

/// One look at the mailbox: new mail read once, then drafted a reply if it passes the email
/// bot's screen, and logged if it is a receipt and receipts are being logged.
pub(super) async fn email_look(handle: &tauri::AppHandle, data_dir: &std::path::Path, settings: &core::settings::Settings, list: &[String]) {
    let replies = settings.email_inbound && !list.is_empty();
    let receipts = settings.expense_intake;
    // The drafter first: one that could act is never handed a stranger's email. A message is
    // read once, so while replies are paused the mailbox waits for both readers — nothing is
    // skipped.
    if replies {
        if let Some(problem) = core::email_bot::drafter_problem(&core::personas::PersonaRegistry::load(data_dir)) {
            log_line(handle, &format!("email bot paused: {problem}"));
            return;
        }
    }
    // Past either reader's daily cap the mailbox waits until tomorrow, for the same reason.
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let state = integrations::email::load_state(data_dir);
    if (replies && integrations::email::drafted_on(&state, &today) >= core::email_bot::DAILY_CAP)
        || (receipts && integrations::email::extracted_on(&state, &today) >= core::expenses::DAILY_CAP)
    {
        return;
    }
    let dir = data_dir.to_path_buf();
    let mail = match tokio::task::spawn_blocking(move || integrations::email::poll_new(&dir, core::email_bot::PER_LOOK)).await {
        Ok(Ok(mail)) => mail,
        Ok(Err(e)) => return log_line(handle, &format!("email bot: couldn't read mail: {e}")),
        Err(e) => return log_line(handle, &format!("email bot: the mail task failed: {e}")),
    };
    let own = integrations::email::creds().map(|c| c.address).unwrap_or_default();
    for m in mail {
        if receipts && core::expenses::looks_like_receipt(&m) {
            log_receipt(handle, data_dir, settings, &m, &today).await;
        }
        if !replies {
            continue;
        }
        match core::email_bot::screen(&m, &own, list) {
            Err(skip) => log_line(handle, &format!("email bot: no draft for message {} — {}", m.uid, skip.reason())),
            Ok(()) => {
                if let Err(e) = answer_email(handle, data_dir, &m, &today).await {
                    log_line(handle, &format!("email bot: couldn't draft a reply to message {}: {e}", m.uid));
                }
            }
        }
    }
}
