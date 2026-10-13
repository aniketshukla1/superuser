//! Journal: the memory timeline (daily → core → dream diaries)

use super::*;

pub(super) fn journal(ctx: &Arc<ApiContext>) -> crate::core::journal::Journal {
    use tauri::Manager;
    crate::core::journal::Journal::new(&ctx.app.state::<crate::AppState>().memory.vault_path)
}

/// `GET /v1/journal` — the whole timeline layer in one round trip: the distilled
/// core memory, the dated daily summaries it came from, and the diary of what each
/// consolidation changed.
pub(super) async fn journal_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let j = journal(&ctx);
    let core = j.read_core();
    let nightly = ctx.app.state::<crate::AppState>().settings.lock().unwrap().deep_dream_enabled;
    let settings = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({
        "core": core,
        "core_entries": core.lines().filter(|l| l.trim_start().starts_with('-')).count(),
        "dailies": j.dailies(),
        "recaps": j.recaps(),
        "dreams": j.dreams(),
        "evolutions": j.evolutions(),
        "nightly_enabled": nightly,
        "evolution_enabled": settings.self_evolution_enabled,
        // Whether there's a pass to put back, so the UI can offer undo only when
        // undoing would do something.
        "can_undo": crate::core::evolution::latest_snapshot(&data_dir).is_some(),
        // What reviews have learned, as opposed to what they changed. Shipped here
        // rather than behind its own endpoint because the Brain view shows it beside
        // the rest of the timeline and one round trip is the whole point of `/journal`.
        "wiki": crate::core::wiki::overview(&ctx.app.state::<crate::AppState>().memory.vault_path),
    }))
}

/// `POST /v1/journal/undo` — put back the files the most recent self-evolution
/// pass changed. Every pass snapshots first, so this is exact rather than a guess,
/// and it works for files the pass created as well as ones it edited.
pub(super) async fn journal_undo(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let (data_dir, vault) = (state.data_dir.clone(), state.memory.vault_path.clone());
    let Some(snap) = crate::core::evolution::latest_snapshot(&data_dir) else {
        return Json(json!({ "ok": false, "error": "nothing to undo" }));
    };
    let scope = crate::core::evolution::Scope::for_review(&data_dir, &vault);
    let n = crate::core::evolution::restore(&snap, &scope);
    // The undo is asymmetric on purpose. The files go back; the wiki keeps the row,
    // flipped to `undone`, and every later review is shown it. Without this the next
    // pass sees exactly the inputs that produced the edit just rejected, and writes
    // it again.
    let rejected = crate::core::wiki::Wiki::new(&vault).mark_undone(&snap.id);
    // Consumed: undoing twice must not silently re-apply an older snapshot.
    crate::core::evolution::forget_snapshot(&data_dir, &snap.id);
    Json(json!({ "ok": true, "restored": n, "rejected": rejected }))
}

/// `POST /v1/journal/dream` — consolidate now over the last `days` of daily memory
/// (default 3). `days: 30` folds a whole backlog in one pass, which is what you
/// want the first time, when dailies exist but the core has never been distilled.
pub(super) async fn journal_dream(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use crate::core::journal::{deep_dream, DreamOutcome};
    use tauri::Manager;
    let settings = ctx.app.state::<crate::AppState>().settings.lock().unwrap().clone();
    let j = journal(&ctx);
    // Asking for it means "do it now". The unchanged-input guard exists to stop the
    // *scheduler* paying for a no-op; leaving it in would make the button do nothing.
    j.record_fingerprint(0);
    let days = body["days"].as_u64().unwrap_or(3) as usize;
    Json(match deep_dream(&settings, &j, days).await {
        DreamOutcome::Consolidated { entries, diary } => json!({ "status": "consolidated", "entries": entries, "diary": diary }),
        DreamOutcome::Skipped(why) => json!({ "status": "skipped", "reason": why }),
        DreamOutcome::Failed(e) => json!({ "status": "failed", "reason": e }),
    })
}

/// `PUT /v1/journal/core` — edit the core memory by hand. It's a plain markdown
/// file the user owns, and a wrong remembered "fact" should be a two-second edit
/// rather than an argument with the assistant.
pub(super) async fn journal_set_core(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    match journal(&ctx).write_core(body["body"].as_str().unwrap_or("")) {
        Ok(()) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}
