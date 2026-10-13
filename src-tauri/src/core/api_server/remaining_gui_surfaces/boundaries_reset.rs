use super::super::*;

/// `POST /v1/boundaries/reset` — back to the default, which refuses nothing. The way
/// out of an unreadable file that doesn't need a text editor.
pub(in crate::core::api_server) async fn boundaries_reset(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::core::boundaries::reset(&dir);
    Json(json!({ "ok": true, "boundaries": crate::core::boundaries::Boundaries::default() }))
}
