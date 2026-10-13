use super::*;

/// The nightly consolidation: distills the day's evidence — the turns that fell
/// out of conversations, as they were said, not the recaps of them — into the core
/// memory the assistant carries into every conversation, and writes a dream diary
/// recording what changed. Runs at 23:55 local; skips silently when there is
/// nothing new, and never blocks a conversation.
pub(super) fn start_dream_scheduler(handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    core::supervise::keep_running("the nightly memory pass", handle, move |handle| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(seconds_until_nightly())).await;

            let (settings, vault) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.memory.vault_path.clone())
            };
            if !settings.deep_dream_enabled {
                continue;
            }
            // 3am is usually quiet, but "usually" is not "always" — see
            // `core::modelgate`. Consolidation is the longest model call the app
            // makes unattended, and someone working late should not lose their next
            // turn to it.
            core::modelgate::wait_for_quiet().await;

            // Vault maintenance, before the journal pass and without a model.
            // Consolidation has always been about the journal; the vault is the
            // store that actually grows without bound, filled by six connectors on
            // a scheduler with no ceiling and nothing that ever retires anything.
            //
            // Both passes are deterministic and both are reported: a maintenance
            // step that quietly moves the user's files is one nobody would trust,
            // and the log line is what makes it reviewable the next morning.
            {
                let engine = core::memory::MemoryEngine::new(vault.clone());
                let merged = engine.dedup();
                if !merged.is_empty() {
                    log_line(&handle, &format!("vault: {} duplicate memories collapsed", merged.len()));
                }
                if settings.archive_after_days > 0 {
                    let filed = engine.archive_stale(settings.archive_after_days);
                    if !filed.is_empty() {
                        log_line(
                            &handle,
                            &format!(
                                "vault: {} synced memories older than {} days moved to vault/archive (nothing deleted)",
                                filed.len(),
                                settings.archive_after_days
                            ),
                        );
                    }
                }
            }

            let journal = core::journal::Journal::new(&vault);
            match core::journal::deep_dream(&settings, &journal, 1).await {
                core::journal::DreamOutcome::Consolidated { entries, diary } => {
                    log_line(&handle, &format!("deep dream: core memory now {entries} entries"));
                    if let Some(main) = handle.get_webview_window("main") {
                        let _ = main.emit("dream-done", serde_json::json!({ "entries": entries, "diary": diary }));
                    }
                }
                core::journal::DreamOutcome::Skipped(why) => log_line(&handle, &format!("deep dream skipped: {why}")),
                core::journal::DreamOutcome::Failed(e) => log_line(&handle, &format!("deep dream failed: {e}")),
            }
        }
    });
}

/// The subconscious: the loop that evaluates rather than fires.
///
/// Every `subconscious_interval_min` it asks the local model whether anything that
/// changed since the last look matters to one of the user's standing watches. A
/// tick over an unchanged vault makes no model call at all, which is what makes it
/// affordable to leave on.
///
/// Ticks never overlap — the loop is a single task that sleeps between passes, so a
/// slow model call delays the next tick instead of stacking a second one on top of
/// it. A tick that fails leaves its deltas unconsumed and tries again next time.
pub(super) fn start_subconscious_scheduler(handle: tauri::AppHandle) {
    use tauri::Manager;
    core::supervise::keep_running("the subconscious pass", handle, move |handle| async move {
        // Let the app settle: startup is the worst moment to take the local model.
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        loop {
            let (settings, data_dir, vault) = {
                let state = handle.state::<AppState>();
                let s = state.settings.lock().unwrap().clone();
                (s, state.data_dir.clone(), state.memory.vault_path.clone())
            };
            // Re-read every pass so a change in Settings takes effect without a
            // restart, and a disabled loop costs one wakeup rather than nothing at
            // all (it has to wake to notice it was switched back on).
            let wait = settings.subconscious_interval_min.max(5) as u64 * 60;
            if settings.subconscious_enabled {
                // Not while the user is talking — see `core::modelgate`. Waited for
                // AFTER the enabled check so a switched-off loop still just sleeps,
                // and BEFORE the tick so the wait is not counted against the interval.
                core::modelgate::wait_for_quiet().await;
                let memory = core::memory::MemoryEngine::new(vault);
                let r = core::subconscious::tick(&settings, &data_dir, &memory).await;
                // Quiet passes are the overwhelming majority, so only say something
                // when there was something to say.
                if r.evaluated > 0 || r.failed > 0 {
                    log_line(
                        &handle,
                        &format!(
                            "subconscious: evaluated {}, acted {}, escalated {}, failed {}",
                            r.evaluated, r.acted, r.escalated, r.failed
                        ),
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    });
}
