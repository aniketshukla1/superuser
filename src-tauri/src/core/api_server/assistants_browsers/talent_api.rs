//! Talent Scout's settings: the ATS export it checks against, the person's outreach voice, and
//! the shortlist it keeps.

use super::super::*;

/// For Settings: the export and how many people it holds (or why it can't be read), the voice
/// sample, and how many are shortlisted.
pub(in crate::core::api_server) fn talent_status(settings: &crate::core::settings::Settings, data_dir: &std::path::Path) -> Value {
    use crate::core::talent;
    let path = settings.talent_applicants.trim();
    let (people, error) = match path {
        "" => (Value::Null, Value::Null),
        p => match talent::load_export(p) {
            Ok(x) => (json!(x.len()), Value::Null),
            Err(e) => (Value::Null, json!(sentence(&e))),
        },
    };
    json!({
        "applicants": path,
        "people": people,
        "error": error,
        "voice": settings.talent_voice,
        "shortlisted": talent::load_shortlist(data_dir).len(),
    })
}

/// Capitalised and ended: a message for Settings.
fn sentence(e: &str) -> String {
    let mut c = e.chars();
    let s = c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default();
    if s.ends_with('.') { s } else { format!("{s}.") }
}

/// The two settings `POST /v1/talent` may change. An export that can't be read is refused, not
/// saved: a check against it would check against nobody.
pub(in crate::core::api_server) fn talent_patch(body: &Value) -> Result<Value, String> {
    let mut patch = serde_json::Map::new();
    if let Some(p) = body.get("talent_applicants").and_then(Value::as_str) {
        if !p.trim().is_empty() {
            crate::core::talent::load_export(p).map_err(|e| sentence(&e))?;
        }
        patch.insert("talent_applicants".into(), json!(p));
    }
    if let Some(v) = body.get("talent_voice").and_then(Value::as_str) {
        patch.insert("talent_voice".into(), json!(v));
    }
    Ok(Value::Object(patch))
}

pub(in crate::core::api_server) async fn talent_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let state = ctx.app.state::<crate::AppState>();
    let settings = state.settings.lock().unwrap().clone();
    Json(talent_status(&settings, &state.data_dir))
}

pub(in crate::core::api_server) async fn talent_set(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let patch = match talent_patch(&body) {
        Ok(p) => p,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let state = ctx.app.state::<crate::AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.apply_patch(&patch);
        s.save(&state.data_dir);
        s.clone()
    };
    let mut status = talent_status(&settings, &state.data_dir);
    status["ok"] = json!(true);
    Json(status)
}

/// `POST /v1/talent/pick` — the system's file picker, for the export. Saving is a separate call,
/// so the checks on it live in one place.
pub(in crate::core::api_server) async fn talent_pick(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.app.dialog().file().add_filter("CSV", &["csv"]).pick_file(move |f| {
        let _ = tx.send(f.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()));
    });
    let picked = tokio::task::spawn_blocking(move || rx.recv().ok().flatten()).await.ok().flatten();
    Json(json!({ "path": picked }))
}

/// `POST /v1/talent/forget` — everyone on the shortlist, forgotten. Written to the trail.
pub(in crate::core::api_server) async fn talent_forget(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    match crate::core::talent::forget_shortlist(&dir) {
        Ok(n) => {
            crate::core::audit::log(&dir, "talent", "user", "shortlist", &format!("forgot {n} shortlisted"), true);
            Json(json!({ "ok": true, "forgotten": n }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}
