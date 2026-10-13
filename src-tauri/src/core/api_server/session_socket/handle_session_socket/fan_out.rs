use super::*;

/// One sub-thread per task, each run in the background in order; the parent gets a
/// summary and the list of threads.
pub(super) async fn fan_out(ctx: &Arc<ApiContext>, tx: &mut WsSink, st: &crate::core::sessions::SessionStore, id: &String, current: &crate::core::sessions::Session, tasks: &[String], model: &Option<String>) {
        let children: Vec<(String, String)> = tasks
            .iter()
            .map(|task| {
                let mut child = crate::core::sessions::Session::new(&current.agent, &current.workspace, &current.model);
                child.parent_id = Some(id.clone());
                child.title = task.clone();
                st.save(&child);
                (child.session_id.clone(), task.clone())
            })
            .collect();

        let summary = format!(
            "I split these into {} threads and started working on each one.",
            children.len()
        );
        let mut m = crate::core::sessions::Message::new("assistant", summary.clone());
        m.items = children
            .iter()
            .map(|(cid, title)| json!({ "kind": "thread", "session_id": cid, "title": title }))
            .collect();
        st.append(&id, m);

        let _ = send(tx, json!({ "type": "assistant_message", "data": { "text": summary } })).await;
        let _ = send(tx, json!({
            "type": "threads_spawned",
            "data": { "threads": children.iter().map(|(cid, t)| json!({ "session_id": cid, "title": t })).collect::<Vec<_>>() }
        })).await;
        let _ = send(tx, json!({ "type": "turn_done", "data": { "artifacts": [], "todos": [] } })).await;

        // Run the children in the background, SEQUENTIALLY: firing N
        // concurrent turns at one local model thrashes it and finishes
        // everything slower than doing them in order.
        let app = ctx.app.clone();
        let dir = {
            use tauri::Manager;
            ctx.app.state::<crate::AppState>().data_dir.clone()
        };
        let persona = current.agent.clone();
        let model_for_children = model.clone();
        tauri::async_runtime::spawn(async move {
            use tauri::Manager;
            let store = crate::core::sessions::SessionStore::new(&dir);
            for (cid, task) in children {
                store.append(&cid, crate::core::sessions::Message::new("user", task.clone()));
                let state = app.state::<crate::AppState>();
                let out = crate::commands::run_agent(
                    &app,
                    &state,
                    persona.clone(),
                    task.clone(),
                    Some(cid.clone()),
                    model_for_children.clone(),
                    None,
                )
                .await;
                let reply = match out {
                    Ok(o) => o.answer,
                    Err(e) => format!("I couldn't finish this one: {e}"),
                };
                store.append(&cid, crate::core::sessions::Message::new("assistant", reply));
            }
        });
}
