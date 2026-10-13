use super::*;

/// Read one receipt into the expense ledger, with the model on this machine
/// (`core::expenses::extract`). Every read counts toward the day's cap, whatever it found.
pub(super) async fn log_receipt(
    handle: &tauri::AppHandle,
    data_dir: &std::path::Path,
    settings: &core::settings::Settings,
    m: &integrations::email::Inbound,
    today: &str,
) {
    let read = core::expenses::extract(settings, m, today).await;
    integrations::email::note_extracted(data_dir, today);
    match read {
        Ok(Some(mut e)) => {
            e.id = format!("r{}", core::inbox::short_id());
            e.added = chrono::Local::now().to_rfc3339();
            match core::expenses::append(data_dir, e.clone()) {
                Ok(core::expenses::Added::Logged) => {
                    core::audit::log(data_dir, "expenses", core::expenses::ASSISTANT, &e.source, &format!("logged receipt {}", e.id), true);
                    log_line(handle, &format!("expenses: logged receipt {} from message {}", e.id, m.uid));
                }
                Ok(core::expenses::Added::AlreadyLogged) => {}
                Err(err) => log_line(handle, &format!("expenses: couldn't log message {}: {err}", m.uid)),
            }
        }
        Ok(None) => log_line(handle, &format!("expenses: message {} isn't a receipt after all", m.uid)),
        Err(err) => log_line(handle, &format!("expenses: couldn't read message {} as a receipt: {err}", m.uid)),
    }
}

/// Draft a reply to one email and park it in the Inbox. The exchange is kept as an ordinary
/// conversation under the Email Assistant — one per thread — so you can read what arrived and
/// what was drafted, and carry on from there.
pub async fn answer_email(
    handle: &tauri::AppHandle,
    data_dir: &std::path::Path,
    m: &integrations::email::Inbound,
    today: &str,
) -> Result<(), String> {
    use tauri::Manager;
    let sid = core::email_bot::session_id(m);
    let store = core::sessions::SessionStore::new(data_dir);
    if store.get(&sid).is_none() {
        let model = handle.state::<AppState>().settings.lock().unwrap().ollama_model.clone();
        let mut s = core::sessions::Session::with_id(&sid, core::email_bot::ASSISTANT, &data_dir.to_string_lossy(), &model);
        s.origin = Some("email".into());
        let about = if m.subject.trim().is_empty() { m.from_addr.as_str() } else { m.subject.trim() };
        s.title = format!("Email · {}", about.chars().take(80).collect::<String>());
        store.save(&s);
    }
    // The thread so far, then this message: a reply to a reply is drafted knowing both.
    let history: Vec<commands::HistMsg> = store
        .get(&sid)
        .map(|s| s.history_wired())
        .unwrap_or_default()
        .into_iter()
        .map(|(role, content, wire)| commands::HistMsg { role, content, wire })
        .collect();
    let prompt = core::email_bot::prompt(m);
    store.append(&sid, core::sessions::Message::new("user", prompt.clone()));

    let answer = {
        let state = handle.state::<AppState>();
        commands::run_agent(handle, &state, core::email_bot::ASSISTANT.to_string(), prompt, Some(sid.clone()), None, Some(history)).await
    };
    let emit = |event: &str| {
        if let Some(main) = handle.get_webview_window("main") {
            use tauri::Emitter;
            let _ = main.emit(event, serde_json::json!({ "session": sid }));
        }
    };
    let answer = match answer {
        Ok(out) => out.answer,
        Err(e) => {
            store.append(&sid, core::sessions::Message::new("assistant", format!("I couldn't draft a reply: {e}")));
            emit("sessions-changed");
            return Err(e);
        }
    };
    let Some(draft) = core::email_bot::draft_of(&answer) else {
        store.append(&sid, core::sessions::Message::new("assistant", answer));
        emit("sessions-changed");
        return Ok(());
    };
    // Parked here, not sent by the model: every reply meets a person first.
    let target = format!("email:{}", m.from_addr);
    let ask = core::inbox::Inbox::load(data_dir).park(core::inbox::Ask::new(
        &sid,
        None,
        "send_message",
        &target,
        &core::email_bot::ask_summary(m, &draft),
        core::email_bot::ask_args(m, &draft),
    ));
    integrations::email::note_drafted(data_dir, today);
    store.append(
        &sid,
        core::sessions::Message::new("assistant", format!("{draft}\n\n— Drafted, not sent. It's waiting in your Inbox for approval.")),
    );
    core::audit::log(data_dir, "email-bot", core::email_bot::ASSISTANT, &target, &format!("drafted a reply ({})", ask.id), true);
    emit("sessions-changed");
    Ok(())
}
