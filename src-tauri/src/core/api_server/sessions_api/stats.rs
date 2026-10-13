use super::super::*;

pub(in crate::core::api_server) fn store(ctx: &ApiContext) -> crate::core::sessions::SessionStore {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let dir = state.data_dir.clone();
    crate::core::sessions::SessionStore::new(&dir)
}

pub(in crate::core::api_server) async fn sessions_list(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    let items: Vec<Value> = store(&ctx)
        .list()
        .iter()
        .map(|s| {
            let mut v = s.info();
            // "working" drives the sidebar's live dot — true even when no socket is
            // attached, because the turn keeps running after you switch away.
            if is_running(&s.session_id) {
                v["liveness"] = json!("working");
            }
            v
        })
        .collect();
    // The GUI reads `.sessions` off this response; a bare array reads as empty.
    Json(json!({ "sessions": items }))
}

/// What this workspace has actually done, for the panel a new chat opens on.
///
/// Every figure here is derived from what is genuinely on disk, and the set is chosen by
/// that constraint rather than by what would look impressive. In particular there is no
/// token count: the backend has never emitted a usage sidecar (the only `input_tokens` in
/// this crate is a string inside a codex_cli test fixture), so a "total tokens" figure would
/// have to be invented. A card that reads 0 forever is worse than a card that is not there.
///
/// Days come from session timestamps, not message ones. `Message.at` exists but defaults to
/// 0, and the sessions already on disk were written before anything set it — so a per-message
/// histogram would report that all history happened at the epoch. A session's created_at is
/// a day you worked; that is the honest grain available.
///
/// `range` is `all` (default), `30d` or `7d`.
pub(in crate::core::api_server) async fn stats(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let days_back: Option<i64> = match q.get("range").map(String::as_str) {
        Some("7d") => Some(7),
        Some("30d") => Some(30),
        _ => None,
    };
    let now = chrono::Local::now();
    let cutoff = days_back.map(|d| (now - chrono::Duration::days(d)).timestamp() as f64);

    let all = store(&ctx).list();
    let sessions: Vec<&crate::core::sessions::Session> = all
        .iter()
        .filter(|s| !s.session_id.starts_with("__"))
        .filter(|s| cutoff.is_none_or(|c| s.updated_at >= c || s.created_at >= c))
        .collect();
    Json(compute_stats(&sessions, now))
}
