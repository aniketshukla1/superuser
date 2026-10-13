use super::*;

/// A message typed into the chat: a context command, an `@brain` question, a list to fan
/// out, or a turn.
#[allow(clippy::too_many_arguments)]
pub(super) async fn on_user_message(ctx: &Arc<ApiContext>, tx: &mut WsSink, rx: &mut WsStream, st: &crate::core::sessions::SessionStore, id: &String, session: &crate::core::sessions::Session, frame: &Value) {
    let text = frame["text"].as_str().unwrap_or("").to_string();
    if text.trim().is_empty() {
        return;
    }
    let model = frame["model"].as_str().filter(|m| !m.is_empty()).map(String::from);
    let current = st.get(&id).unwrap_or_else(|| session.clone());
    crate::trace!(
        "chat",
        "user_message session={id} frame_model={:?} stored_model={:?} text={}",
        model,
        current.model,
        crate::core::trace::preview(&text)
    );

    // Context commands are meta, not turns: they answer immediately and
    // neither the command nor its answer is written to the transcript —
    // a report about context usage that itself consumed context would be
    // its own worst example.
    if let Some(reply) = handle_context_command(&ctx, &st, &current, text.trim()).await {
        let _ = send(tx, json!({ "type": "assistant_message", "data": { "text": reply } })).await;
        let _ = send(tx, json!({ "type": "turn_done", "data": { "artifacts": [], "todos": [] } })).await;
        return;
    }

    // `history_wired`, not `history`: each turn's own tool traffic rides
    // along so the next turn can see what this one already did. What is
    // usable depends on the model about to run — `core::transcript` decides.
    let history: Vec<crate::commands::HistMsg> = current
        .history_wired()
        .into_iter()
        .map(|(role, content, wire)| crate::commands::HistMsg { role, content, wire })
        .collect();
    // First turn: this is when the session becomes real on disk.
    //
    // And it must record the model the turn is ACTUALLY running on. A
    // session is held in memory until now (persisting on connect would
    // litter the sidebar with blank rows), which means `set_model` found
    // nothing to write to and quietly did nothing — so a model chosen in
    // a new chat, before its first message, was never stored. The turn
    // still used it, because the composer carries the model on every
    // message; it was only the record that forgot, and the chat reverted
    // to the default the next time it was opened.
    let mut current = current;
    if let Some(m) = model.as_deref().filter(|m| !m.is_empty()) {
        current.model = m.to_string();
    }
    if st.get(&id).is_none() {
        st.save(&current);
    } else if !current.model.is_empty() {
        // Already on disk: keep its stored model in step with the turn,
        // so a pick made mid-conversation survives a reload too.
        if let Some(mut on_disk) = st.get(&id) {
            if on_disk.model != current.model {
                on_disk.model = current.model.clone();
                st.save(&on_disk);
            }
        }
    }
    st.append(&id, crate::core::sessions::Message::new("user", text.clone()));

    // `@brain …` — a question about what the brain holds, answered from
    // all four memory stores at once.
    //
    // Unlike `/context`, this IS the conversation: the question and its
    // answer both stay in the transcript (the user message is already
    // appended above), because the next turn will be a follow-up on the
    // answer far more often than not. It runs here rather than as a full
    // agent turn because it is a single retrieval and a single
    // completion — putting it through the tool loop would buy nothing
    // and cost the user a multi-step run.
    //
    // Handled server-side, so it also works typed by hand, over the
    // `/v1` API, and from any front end that never heard of the "@" menu.
    if let Some(question) = crate::core::brain::parse_mention(&text) {
        let _ = send(tx, json!({ "type": "turn_start", "data": {} })).await;
        let reply = brain_reply(&ctx, tx, &question, model.as_deref()).await;
        st.append(&id, crate::core::sessions::Message::new("assistant", reply.clone()));
        let _ = send(tx, json!({ "type": "assistant_message", "data": { "text": reply } })).await;
        let _ = send(tx, json!({ "type": "turn_done", "data": { "artifacts": [], "todos": [] } })).await;
        return;
    }

    // `@expenses …` — a message for another assistant, which answers it here, as itself
    // (its own prompt, tools and connected apps): ChatGPT-style mentions (owner,
    // 2026-10-13). Only this turn changes hands — the conversation stays its own
    // assistant's — and `run_turn` marks the reply with who gave it. The history it reads
    // carries no tool traffic: those calls were another assistant's, with other tools.
    // Parsed here, like `@brain`, so a mention typed by hand works the same as the menu.
    if text.trim_start().starts_with('@') {
        let dir = { use tauri::Manager; ctx.app.state::<crate::AppState>().data_dir.clone() };
        let reg = crate::core::personas::PersonaRegistry::load(&dir);
        if let Some((who, asked)) = crate::core::coworker::mentioned(&reg, &current.agent, &text) {
            let own = reg.manifest(&current.agent).map(|m| m.name.clone()).unwrap_or_else(|| current.agent.clone());
            let mut as_them = current.clone();
            as_them.agent = who.id.clone();
            let history = history.into_iter().map(|h| crate::commands::HistMsg { wire: None, ..h }).collect();
            let brief = crate::core::coworker::mention_brief(&own, &who.name, &asked);
            run_turn(ctx, tx, rx, st, id, &as_them, brief, model, history).await;
            return;
        }
    }

    // A skill the user picked from the "/" menu. Validated rather than
    // trusted — the field comes from the client, and framing a turn
    // around a skill this session muted, or one nobody installed, would
    // be acting on a menu that does not exist.
    //
    // The instructions ride the message sent to the model, NOT the one
    // written to the transcript above: the user typed a sentence, and a
    // transcript that shows them pasting a skill body is a transcript
    // that lies about what they said.
    let forced: Option<(String, String)> =
        frame["skill"].as_str().map(str::trim).filter(|s| !s.is_empty()).and_then(|want| {
            let dir = { use tauri::Manager; ctx.app.state::<crate::AppState>().data_dir.clone() };
            let store = crate::core::skills::SkillStore::load(&dir);
            let muted = !crate::core::session_skills::SessionSkills::load(&dir).is_enabled(&id, want);
            let allowed = allowed_skills_for(&dir, &id, None);
            // Switched on and on the assistant's list — `usable` — and not muted here.
            store
                .usable(want, &allowed)
                .filter(|_| !muted)
                .map(|s| (s.name.clone(), s.instructions.clone()))
        });
    let text = match &forced {
        Some((name, body)) => format!(
            "The user invoked the `{name}` skill for this message. Follow it — you do not need \
             to load it, its instructions are here.\n\n<skill name=\"{name}\">\n{body}\n</skill>\n\n{text}"
        ),
        None => text,
    };

    // Fan-out: a request written as a list becomes one sub-thread per
    // task, each working independently (the sidebar nests them under
    // this session). Guarded by `parent_id.is_none()` so a sub-thread
    // can never fan out again.
    let tasks = crate::core::sessions::split_tasks(&text);
    if tasks.len() >= 2 && current.parent_id.is_none() {
        fan_out(ctx, tx, st, id, &current, &tasks, &model).await;
        return;
    }
    run_turn(ctx, tx, rx, st, id, &current, text, model, history).await;
}
