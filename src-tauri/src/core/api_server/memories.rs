//! memories (the Memory Tree, made visible)

use super::*;

pub(super) fn memory_engine(ctx: &ApiContext) -> crate::core::memory::MemoryEngine {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    crate::core::memory::MemoryEngine::new(state.memory.vault_path.clone())
}

/// `GET /v1/memory` — every saved memory, newest first, with an optional `q`
/// filter. The body is omitted from the list: a vault of long notes would make
/// this response enormous for a view that only shows titles until you pick one.
pub(super) async fn memory_list(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let engine = memory_engine(&ctx);
    let needle = q.get("q").map(|s| s.trim().to_lowercase()).unwrap_or_default();
    let mut items: Vec<crate::core::memory::Memory> = engine.list();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    let memories: Vec<Value> = items
        .into_iter()
        .filter(|m| {
            needle.is_empty()
                || m.title.to_lowercase().contains(&needle)
                || m.summary.to_lowercase().contains(&needle)
                || m.category.to_lowercase().contains(&needle)
                || m.tags.iter().any(|t| t.to_lowercase().contains(&needle))
                || m.body.to_lowercase().contains(&needle)
        })
        .map(|m| {
            json!({
                "id": m.id,
                "filename": m.filename,
                "title": m.title,
                "category": m.category,
                "tags": m.tags,
                "summary": m.summary,
                "date": m.date,
                "weight": m.weight,
                "origin": m.origin,
            })
        })
        .collect();
    Json(json!({ "memories": memories }))
}

/// `GET /v1/memory/:filename` — one memory, including its body.
pub(super) async fn memory_get(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Json<Value> {
    let engine = memory_engine(&ctx);
    match engine.list().into_iter().find(|m| m.filename == filename) {
        Some(m) => Json(json!({
            "ok": true,
            "memory": {
                "id": m.id, "filename": m.filename, "title": m.title, "category": m.category,
                "tags": m.tags, "summary": m.summary, "body": m.body, "date": m.date, "weight": m.weight,
                "origin": m.origin,
            }
        })),
        None => Json(json!({ "ok": false, "error": format!("no memory called {filename}") })),
    }
}

/// `DELETE /v1/memory/:filename` — forget one. Deliberately available: a memory
/// store you can't correct is one you can't trust.
pub(super) async fn memory_delete(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Json<Value> {
    let ok = memory_engine(&ctx).delete(&filename);
    if ok {
        use tauri::Manager;
        let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
        crate::core::audit::log(&dir, "memory", "user", &filename, "deleted", true);
    }
    Json(json!({ "ok": ok }))
}
