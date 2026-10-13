use super::super::*;

pub(in crate::core::api_server) async fn persona_update(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mut reg = registry(&ctx);
    // A body naming no field we understand used to answer `ok: true` having changed
    // nothing — and `:id` matches anything, so `POST /v1/personas/install` (a route
    // that does not exist) landed here as an assistant called "install" and was told
    // it succeeded. Refuse an id we do not have.
    if reg.manifest(&id).is_none() {
        return Json(json!({ "ok": false, "error": format!("unknown assistant: {id}") }));
    }
    let mut ok = true;
    if let Some(v) = body["enabled"].as_bool() {
        ok &= reg.set_enabled(&id, v).is_ok();
    }
    if let Some(v) = body["surfaced"].as_bool() {
        ok &= reg.set_surfaced(&id, v).is_ok();
    }
    if body["default"].as_bool().unwrap_or(false) {
        ok &= reg.set_default(&id).is_ok();
    }
    // The two the detail page displayed and could not change. An invalid mode is
    // reported rather than swallowed into `ok: false` with no reason — "it didn't
    // save" without saying why is the thing that sends someone reading logs.
    let mut error: Option<String> = None;
    if let Some(v) = body["default_permission_mode"].as_str() {
        if let Err(e) = reg.set_mode(&id, v) {
            ok = false;
            error = Some(e);
        }
    }
    if let Some(v) = body["subagents"].as_array() {
        let roster: Vec<String> =
            v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
        if let Err(e) = reg.set_roster(&id, roster) {
            ok = false;
            error = Some(e);
        }
    }
    // Who it may hand work to. An id it can't reach is dropped rather than refused — see
    // `set_asks`.
    if let Some(v) = body["asks"].as_array() {
        let who: Vec<String> = v.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
        if let Err(e) = reg.set_asks(&id, who) {
            ok = false;
            error = Some(e);
        }
    }
    // Where it runs. A refused address comes back with the floor's reason; clearing the
    // address clears its token too, because a credential for an agent nothing talks to is
    // one more thing to leak and nothing to use.
    if let Some(v) = body["remote"].as_str() {
        match reg.set_remote(&id, v) {
            Err(e) => {
                ok = false;
                error = Some(e);
            }
            Ok(()) if v.trim().is_empty() => {
                crate::core::secrets::clear_agent_token(&id);
            }
            Ok(()) => {}
        }
    }
    if let Some(v) = body["remote_token"].as_str() {
        if v.trim().is_empty() {
            crate::core::secrets::clear_agent_token(&id);
        } else {
            crate::core::secrets::set_agent_token(&id, v.trim());
        }
    }
    // Its own browser (Settings ▸ Browser). Taking it away closes the one it has open.
    if let Some(v) = body["browser"].as_bool() {
        if let Err(e) = reg.set_browser(&id, v) {
            ok = false;
            error = Some(e);
        } else if !v {
            if let Some(c) = crate::core::agentbrowser::computer::existing(&id) {
                c.stop("Its browser was taken away in Settings.").await;
            }
        }
    }
    match error {
        Some(e) => Json(json!({ "ok": false, "error": e })),
        None => Json(json!({ "ok": ok })),
    }
}
