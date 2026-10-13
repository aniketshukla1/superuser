use super::*;

/// Create the floating, transparent, always-on-top mascot window and park it at
/// the bottom-right of the primary monitor.
pub(super) fn spawn_mascot(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let win = WebviewWindowBuilder::new(app, "mascot", WebviewUrl::App("index.html?mascot".into()))
        .title("")
        .inner_size(190.0, 220.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false) // hidden while the main app is open; shown when it minimizes
        .build();

    if let Ok(win) = win {
        if let Ok(Some(monitor)) = win.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let w = (size.width as f64 / scale) as i32;
            let h = (size.height as f64 / scale) as i32;
            let _ = win.set_position(tauri::LogicalPosition::new((w - 210) as f64, (h - 260) as f64));
        }
    }
}

/// Keep the mascot's visibility in sync with the main window: show the mascot
/// only when the app is minimized or hidden, hide it when the app is up front.
pub(super) fn watch_main_window(handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    std::thread::spawn(move || {
        let mut shown = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(600));
            let main = handle.get_webview_window("main");
            let mascot = handle.get_webview_window("mascot");
            if let (Some(main), Some(mascot)) = (main, mascot) {
                let away = main.is_minimized().unwrap_or(false) || !main.is_visible().unwrap_or(true);
                if away != shown {
                    shown = away;
                    let _ = if away { mascot.show() } else { mascot.hide() };
                    // Tell the mascot whether it's actually on screen so it only
                    // speaks/shows notifications while the app is minimized.
                    let _ = mascot.emit("mascot-visible", away);
                }
            }
        }
    });
}

/// The "subconscious": background auto-sync. Every `sync_interval_min` minutes
/// it syncs every connected source. When new memories arrive it pushes a typed
/// notification to the mascot (spoken while you're away) and tells the main
/// window to refresh its data.
pub(super) fn start_scheduler(handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    // Not in a scratch instance: it holds the real credentials and none of the real state, so
    // this would copy the person's mail and documents into a throwaway folder — and into the
    // harness's memory tasks. See `core::instance::is_scratch`.
    if core::instance::is_scratch() {
        log_line(&handle, "scratch instance: connector sync not started");
        return;
    }
    core::supervise::keep_running("the connector sync", handle, move |handle| async move {
        // Give the app a moment to settle before the first cycle.
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        loop {
            let state = handle.state::<AppState>();
            let interval_min = state.settings.lock().unwrap().sync_interval_min;
            if interval_min == 0 {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }

            let vault = state.memory.vault_path.clone();
            let engine = core::memory::MemoryEngine::new(vault);
            let mut new_total = 0u32;
            let mut highlight: Option<String> = None;

            for id in ["github", "gmail", "calendar", "notion"] {
                if !core::secrets::has_integration_token(integrations::token_key(id)) {
                    continue;
                }
                match integrations::sync_one(id, &engine).await {
                    Ok(out) if out.created > 0 => {
                        new_total += out.created;
                        if highlight.is_none() {
                            highlight = out.titles.first().cloned();
                        }
                        let total = state.registry.list(&state.memory).into_iter().find(|i| i.id == id).map(|i| i.memory_count).unwrap_or(0);
                        state.registry.mark_synced(id, total);
                    }
                    Ok(_) => {}
                    Err(e) => log_line(&handle, &format!("scheduler: {id} sync failed: {e}")),
                }
            }

            if new_total > 0 {
                let msg = match highlight {
                    Some(t) if new_total == 1 => t,
                    Some(t) => format!("{t} (+{} more)", new_total - 1),
                    None => format!("Synced {new_total} new memories"),
                };
                if let Some(mascot) = handle.get_webview_window("mascot") {
                    let _ = mascot.emit("mascot-message", serde_json::json!({ "message": msg, "type": "message" }).to_string());
                }
                if let Some(main) = handle.get_webview_window("main") {
                    let _ = main.emit("data-refresh", new_total);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval_min as u64 * 60)).await;
        }
    });
}

/// Seconds from now until the next occurrence of 23:55 local time.
///
/// Deliberately recomputed each cycle rather than sleeping a fixed 24h: a laptop
/// that was asleep at midnight, or a machine that crossed a DST boundary, would
/// otherwise drift a little further from "late evening" every day until the
/// consolidation was landing at lunchtime.
pub(super) fn seconds_until_nightly() -> u64 {
    use chrono::{Local, NaiveTime, TimeZone};
    let now = Local::now();
    let at = NaiveTime::from_hms_opt(23, 55, 0).expect("23:55 is a valid time");
    let today = now.date_naive().and_time(at);
    let target = if today > now.naive_local() { today } else { now.date_naive().succ_opt().unwrap_or(now.date_naive()).and_time(at) };
    // A local time can be ambiguous or nonexistent across a DST jump; either way the
    // nearest valid instant is close enough for a nightly housekeeping job.
    let target = Local.from_local_datetime(&target).earliest().map(|t| t.timestamp()).unwrap_or_else(|| now.timestamp() + 3600);
    (target - now.timestamp()).max(60) as u64
}
