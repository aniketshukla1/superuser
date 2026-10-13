use super::*;

/// Everything that starts once the app exists: wake-ups from a second launch, the test
/// bridge (dev builds), the tray, the schedulers and watchers, the local API, the sweeps.
pub(super) fn setup(app: &mut tauri::App, wakes: std::sync::mpsc::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    // Somebody opened the app again while this copy was running: this one comes forward.
    {
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            while wakes.recv().is_ok() {
                commands::show_main_window(handle.clone());
            }
        });
    }
    // DEV ONLY: window-management permissions for the automated test bridge.
    //
    // `capabilities/default.json` deliberately does NOT let the webview resize,
    // move, or full-screen its own window, and that is the right shipped
    // posture — the test bridge discovered it by being refused. So this grant
    // is added at RUNTIME under `debug_assertions` and its JSON lives outside
    // `capabilities/`, which Tauri would otherwise compile into every build
    // including release. Two independent gates: this one, and `import.meta.env.DEV`
    // on the webview half (surfaces/gui/src/testbridge.ts).
    #[cfg(debug_assertions)]
    {
        use tauri::Manager; // add_capability lives on the Manager trait
        if let Err(e) = app
            .handle()
            .add_capability(include_str!("../../../testbridge-capability.json"))
        {
            eprintln!("[superuser] test-bridge capability not added: {e}");
        }
    }
    // The menu-bar icon and the app menu. A failure is logged, not fatal: the app works
    // without them, and a tray that couldn't be drawn is no reason to not start.
    if let Err(e) = tray::install(app) {
        eprintln!("[superuser] tray and menu not installed: {e}");
    }
    spawn_mascot(app.handle());
    watch_main_window(app.handle().clone());
    start_scheduler(app.handle().clone());
    // An assistant's browser left idle for half an hour is closed; its sign-ins stay.
    core::supervise::keep_running("the idle-browser sweep", app.handle().clone(), |_| async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            core::agentbrowser::computer::sweep_idle().await;
        }
    });
    start_automation_scheduler(app.handle().clone());
    start_dream_scheduler(app.handle().clone());
    start_evolution_scheduler(app.handle().clone());
    start_subconscious_scheduler(app.handle().clone());
    start_profile_scheduler(app.handle().clone());
    {
        use tauri::Manager;
        core::subagent::ensure_templates(&app.handle().state::<AppState>().data_dir);
    }
    // The trail opens every session by saying which boundaries were in force —
    // so a row read months later can be judged against the rules of its day —
    // and then trims itself, if and only if the user set a retention window.
    {
        use tauri::Manager;
        let state = app.handle().state::<AppState>();
        let retention = state.settings.lock().map(|s| s.audit_retention_days).unwrap_or(0);
        core::boundaries::record_loaded(&state.data_dir);
        core::audit::apply_retention(&state.data_dir, retention);
    }
    start_slack_watcher(app.handle().clone());
    start_telegram_watcher(app.handle().clone());
    start_email_watcher(app.handle().clone());
    start_local_api(app.handle().clone());
    // Sweep sessions that were opened but never used (nothing is in flight
    // at startup, so this can't remove a live one).
    {
        use tauri::Manager;
        let dir = app.handle().state::<AppState>().data_dir.clone();
        let n = core::sessions::SessionStore::new(&dir).prune_empty();
        if n > 0 {
            log_line(app.handle(), &format!("pruned {n} empty session(s)"));
        }
    }
    Ok(())
}
