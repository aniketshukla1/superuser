use super::*;

/// The profile learner. Every few minutes it finds one conversation that has gone
/// quiet and reads **what the user said in it** for lasting preferences.
///
/// Deliberately not folded into the self-evolution scan next door, though both
/// pick an idle session: evolution reads the whole exchange and rewrites skills and
/// core memory on the chat model, this reads only the user's own turns and writes
/// decaying observations on the small one. They are also gated separately, and a
/// user who wants one without the other should get exactly that.
pub(super) fn start_profile_scheduler(handle: tauri::AppHandle) {
    use tauri::Manager;
    core::supervise::keep_running("the profile pass", handle, move |handle| async move {
        // Nothing has gone quiet yet at startup, and the local model has better
        // things to do while the app is still opening.
        tokio::time::sleep(std::time::Duration::from_secs(150)).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;

            let (settings, data_dir, vault) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.data_dir.clone(), state.memory.vault_path.clone())
            };
            if !settings.profile_learning_enabled {
                continue;
            }

            let store = core::sessions::SessionStore::new(&data_dir);

            // Off the async runtime for the same reason as the evolution scan next
            // door — and with the same caveat: this is hygiene, not the cure for the
            // stalls in §0.3.
            let scan = {
                let data_dir = data_dir.clone();
                tokio::task::spawn_blocking(move || {
                    let store = core::sessions::SessionStore::new(&data_dir);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0);
                    store.list().into_iter().find(|s| {
                        if core::api_server::is_running(&s.session_id) {
                            return false; // still mid-turn — they may not have finished the thought
                        }
                        now - s.updated_at >= PROFILE_IDLE_MINUTES * 60.0
                            && s.user_turns_since_profile().len() >= PROFILE_MIN_TURNS
                    })
                })
                .await
            };
            let Some(session) = scan.ok().flatten() else { continue };

            // Same reasoning as the evolution scan next door: this session is idle,
            // the user need not be. See `core::modelgate`.
            core::modelgate::wait_for_quiet().await;

            // Stamped before the pass, like the evolution scan: a pass that fails
            // or finds nothing must not re-read the same turns every two minutes.
            let turns = session.user_turns_since_profile();
            store.mark_profiled(&session.session_id);
            match core::profile::observe_turns(&settings, &data_dir, &session.session_id, &turns).await {
                Ok(l) if l.recorded > 0 => {
                    // PROFILE.md is the readable face of the store — rewritten here
                    // rather than on read, so the file on disk is never a stale
                    // rendering of what the app is actually doing.
                    let learned = core::profile::Learned::load(&data_dir);
                    let _ = learned.write_profile_md(&vault);
                    // The two numbers differ when something the model proposed was
                    // one the user has forgotten. That's the block doing its job,
                    // and it should be visible rather than looking like a miscount.
                    let blocked = l.proposed.saturating_sub(l.recorded);
                    log_line(
                        &handle,
                        &format!(
                            "profile: {} observation(s) from {}{}",
                            l.recorded,
                            session.session_id,
                            if blocked > 0 { format!(" ({blocked} previously forgotten)") } else { String::new() }
                        ),
                    );
                }
                // Silent when there was nothing to learn, which is the common case
                // and not worth a line in the log.
                Ok(_) => {}
                Err(e) => log_line(&handle, &format!("profile learning failed: {e}")),
            }
        }
    });
}

/// Fires due automations. Every ~30s it loads the automation store, runs each due
/// task through the assistant loop (recording a Run + its artifacts), and rolls the
/// task's next fire forward. Notifies the mascot on completion.
/// Injected scheduled-run pairs kept per conversation. A five-minute monitor would
/// otherwise bury everything the user actually said.
pub(super) const SCHEDULED_PAIRS_PER_SESSION: usize = 3;
