use super::super::*;

/// `GET /v1/risk-overrides` — the user's corrections to what the app guessed
/// about a tool's risk, each marked with whether it can actually bite.
pub(in crate::core::api_server) async fn risk_overrides_get(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    Json(json!({ "rules": crate::core::overrides::RiskOverrides::load(&data_dir).view() }))
}

/// `POST /v1/risk-overrides` — replace the whole list.
///
/// Whole-list rather than add/remove: the rules are order-sensitive through
/// specificity, so a client that could only append would have no way to express
/// "this narrow rule beats that broad one".
pub(in crate::core::api_server) async fn risk_overrides_set(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let rules: Vec<(String, String)> = body["rules"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    (
                        r["pattern"].as_str().unwrap_or("").to_string(),
                        r["risk"].as_str().unwrap_or("").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    match crate::core::overrides::RiskOverrides::save(&data_dir, &rules) {
        Ok(saved) => Json(json!({ "ok": true, "rules": saved.view() })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `GET /v1/workspaces/trusted` — the projects whose declared command allowances
/// are switched on, for the Trusted workspaces card.
///
/// The card has been in Settings since the GUI landed, calling this path and
/// getting a 404 — so it rendered "No workspaces are trusted." whatever the truth
/// was. Same shape of bug as `permission_required`: a surface built against a
/// contract nothing on this side implemented.
pub(in crate::core::api_server) async fn workspaces_trusted(AxState(ctx): AxState<Arc<ApiContext>>) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let rows = crate::core::workspace_trust::TrustStore::load(&data_dir).list();
    Json(json!({ "workspaces": rows }))
}

/// `POST /v1/workspaces/trust` — trust a project's declared allowances, or revoke.
///
/// Returns the folder's status afterwards, so the card can render the new row
/// without a second round trip and without guessing at what it now declares.
pub(in crate::core::api_server) async fn workspace_trust_set(
    AxState(ctx): AxState<Arc<ApiContext>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    if path.is_empty() {
        return Json(json!({ "ok": false, "error": "a workspace path is required" }));
    }
    let trusted = body["trusted"].as_bool().unwrap_or(false);
    let store = crate::core::workspace_trust::TrustStore::load(&data_dir);
    if let Err(e) = store.set(&path, trusted) {
        return Json(json!({ "ok": false, "error": e }));
    }
    let status = store.status(&path);
    crate::core::audit::log(
        &data_dir,
        if trusted { "approve" } else { "deny" },
        "user",
        &status.workspace,
        if trusted { "trusted this project's command allowances" } else { "revoked command trust" },
        true,
    );
    let mut out = serde_json::to_value(&status).unwrap_or_else(|_| json!({}));
    out["ok"] = json!(true);
    Json(out)
}

/// Superuser has no cloud account — say so in the shape the GUI expects rather
/// than 404ing, so the sign-in affordances simply stay signed-out.
pub(in crate::core::api_server) async fn cloud_status() -> Json<Value> {
    Json(json!({ "signed_in": false, "email": null, "plan": null, "available": false }))
}

/// Any route we don't serve still answers with parseable JSON. The GUI calls
/// `.json()` unconditionally, so an empty-bodied 404 would crash the whole app —
/// an honest error object keeps the rest of the UI alive.
pub(in crate::core::api_server) async fn not_implemented(uri: axum::http::Uri) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": { "message": format!("{} isn't implemented by Superuser", uri.path()), "type": "not_implemented" },
            // Empty collections under the keys the GUI's list endpoints use, so a
            // caller that expects a list gets an empty one instead of undefined.
            "items": [], "connectors": [], "automations": [], "servers": [],
            "personas": [], "workspaces": [], "entries": [], "sessions": [],
        })),
    )
}
