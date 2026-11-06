use super::*;

/// Self-evolution. Every minute it looks for a conversation that has gone quiet
/// and accumulated enough to be worth reviewing, then reviews exactly one of them.
///
/// One per scan on purpose: a review costs a model call, and on a machine with
/// twenty idle sessions after an upgrade, reviewing them all at once would take
/// over the local model for minutes. They get picked up on successive scans.
pub(super) fn start_evolution_scheduler(handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    core::supervise::keep_running("the evolution pass", handle, move |handle| async move {
        // Nothing is idle at startup, and the first minute is the busiest.
        tokio::time::sleep(std::time::Duration::from_secs(90)).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let (settings, vault, data_dir) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.memory.vault_path.clone(), state.data_dir.clone())
            };
            if !settings.self_evolution_enabled {
                continue;
            }

            let store = core::sessions::SessionStore::new(&data_dir);

            // Hygiene, not a fix for anything currently observed: the scan reads
            // every session's transcript off disk and estimates its token count, and
            // synchronous filesystem work does not belong on the runtime that also
            // serves the HTTP sidecar.
            //
            // Measured 2026-08-21 against 41 real sessions, the whole scan costs
            // **1.57ms** — so if you are here chasing the multi-second sidecar
            // stalls, it is not this. That was tested by
            // reverting to the pre-`spawn_blocking` version, and the stalls did not
            // come back. The cost does grow with session count and transcript
            // length, which is the only reason this is worth keeping.
            //
            // Nothing here is async, so nothing is lost by moving it.
            let scan = {
                let data_dir = data_dir.clone();
                let settings = settings.clone();
                tokio::task::spawn_blocking(move || {
                    let store = core::sessions::SessionStore::new(&data_dir);
                    let budget = core::context::Budget::resolve(
                        &settings,
                        core::providers::resolve(&settings.ollama_model).is_none(),
                    );
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0);
                    store.list().into_iter().find(|s| {
                        if core::api_server::is_running(&s.session_id) {
                            return false; // a turn is still in flight — not idle, whatever the clock says
                        }
                        let est = core::context::estimate_tokens(&s.transcript());
                        core::evolution::due(
                            s,
                            est,
                            budget.max_tokens,
                            settings.self_evolution_idle_minutes,
                            settings.self_evolution_min_turns,
                            now,
                        )
                        .should_run()
                    })
                })
                .await
            };
            let Some(session) = scan.ok().flatten() else { continue };

            // The session being reviewed is idle; the *user* may not be. They are
            // often idle here precisely because they moved to another chat, which is
            // the one that would pay for this. See `core::modelgate`.
            core::modelgate::wait_for_quiet().await;

            // Stamped before the pass: if the review fails or finds nothing, the same
            // turns must not be re-reviewed every single minute.
            store.mark_evolved(&session.session_id);
            match core::evolution::review(&settings, &data_dir, &vault, &session.transcript()).await {
                core::evolution::Outcome::Changed { summary, files, snapshot } => {
                    log_line(&handle, &format!("evolution: {} file(s) changed in {}", files.len(), session.session_id));
                    // Told once, in the session it came from, and only because files
                    // really changed.
                    store.append(
                        &session.session_id,
                        core::sessions::Message::new("assistant", format!("{summary}\n\n_Self-learning pass — run `/undo` to put it back._")),
                    );
                    if let Some(main) = handle.get_webview_window("main") {
                        let _ = main.emit("evolution-done", serde_json::json!({
                            "session": session.session_id, "summary": summary, "files": files, "snapshot": snapshot,
                        }));
                    }
                }
                // Deliberately not told to the user: a pattern is something the next
                // review inherits, not work the user asked for or has to check.
                core::evolution::Outcome::Noted { patterns } => log_line(
                    &handle,
                    &format!("evolution: noted {} pattern(s) from {}", patterns.len(), session.session_id),
                ),
                core::evolution::Outcome::Silent => {}
                core::evolution::Outcome::Failed(e) => log_line(&handle, &format!("evolution failed: {e}")),
            }
        }
    });
}

/// How long a conversation must be untouched before the profile learner reads it.
/// Short, because it reads only what the user typed and nothing it learns takes
/// effect until the next session anyway.
pub(super) const PROFILE_IDLE_MINUTES: f64 = 5.0;

/// User turns that must have accumulated since the last pass. One turn is almost
/// never a preference — it's a request — and a model call per message would be a
/// tax on every conversation.
pub(super) const PROFILE_MIN_TURNS: usize = 2;
