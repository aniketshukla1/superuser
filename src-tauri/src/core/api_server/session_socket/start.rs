use super::super::*;

mod core_routes;
use core_routes::*;

pub async fn start(app: tauri::AppHandle, port: u16) -> Result<RunningApi, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    // Release: a fresh per-launch token, handed to the window in memory — it never
    // touches disk. Dev: reuse a stable token from the (0600) state file, because
    // the GUI's dev server bakes it in at ITS startup, which happens before we
    // could write a new one — a per-launch token would always be one run stale.
    let token = {
        #[cfg(debug_assertions)]
        {
            use tauri::Manager;
            let path = app.state::<crate::AppState>().data_dir.join("sidecar-8765.token");
            match std::fs::read_to_string(&path).map(|s| s.trim().to_string()) {
                Ok(t) if t.len() >= 16 => t,
                _ => URL_SAFE_NO_PAD.encode(rand::random::<[u8; 24]>()),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            URL_SAFE_NO_PAD.encode(rand::random::<[u8; 24]>())
        }
    };
    let ctx = Arc::new(ApiContext { app, token: token.clone() });
    // Taken here because the router consumes `ctx` below, and the token file can only be
    // written once we know which port we actually got.
    #[cfg(debug_assertions)]
    let token_dir = {
        use tauri::Manager;
        ctx.app.state::<crate::AppState>().data_dir.clone()
    };

    // Everything except /v1/health requires the token. The server is loopback-only,
    // but "local" is not "trusted" — any process on this machine could otherwise
    // read the user's sessions.
    let guard = {
        let expected = token.clone();
        axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let expected = expected.clone();
            async move {
                let path = req.uri().path().to_string();
                let ok = path == "/v1/health"
                    || authed(req.headers(), &expected)
                    // Browsers can't set headers on a WebSocket handshake, so the
                    // token rides as a subprotocol (`["superuser", <token>]`).
                    || req
                        .headers()
                        .get("sec-websocket-protocol")
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.split(',').any(|p| p.trim() == expected))
                        .unwrap_or(false);
                if ok {
                    Ok(next.run(req).await)
                } else {
                    Err(unauthorized())
                }
            }
        })
    };

    let router = Router::new()
        .merge(core_routes())
        .route("/v1/watches", get(watches_list).post(watch_create))
        .route("/v1/watches/tick", post(watches_tick))
        .route("/v1/watches/:id", axum::routing::patch(watch_patch).delete(watch_delete))
        .route("/v1/automations", get(automations_list).post(automation_create))
        .route("/v1/inbox", get(inbox_list))
        .route("/v1/inbox/:id/resolve", post(inbox_resolve_api))
        .route(
            "/v1/automations/:id",
            get(automation_get).patch(automation_patch).delete(automation_delete),
        )
        .route("/v1/automations/:id/run", post(automation_run))
        .route("/v1/automations/:id/seen", post(automation_seen))
        .route("/v1/automations/:id/runs/:run_id/finalize", post(automation_finalize))
        .route("/v1/mcp", get(mcp_list_api).post(mcp_add_api))
        .route("/v1/mcp/reload", post(mcp_reload_api))
        .route("/v1/mcp/:name", axum::routing::patch(mcp_patch_api).delete(mcp_delete_api))
        .route("/v1/mcp/:name/tools", get(mcp_tools_api))
        .route("/v1/mcp/:name/connect", post(mcp_connect_api))
        .route("/v1/mcp/:name/signout", post(mcp_signout_api))
        .route("/v1/audit", get(audit_list))
        .route("/v1/audit/verify", get(audit_verify))
        .route("/v1/boundaries", get(boundaries_get).post(boundaries_set))
        .route("/v1/boundaries/check", post(boundaries_check))
        .route("/v1/boundaries/test", post(boundaries_test))
        .route("/v1/boundaries/reset", post(boundaries_reset))
        .route("/v1/computers", get(computers_list))
        .route("/v1/computers/:persona", get(computer_status))
        .route("/v1/computers/:persona/stop", post(computer_stop))
        .route("/v1/computers/:persona/reset", post(computer_reset))
        .route("/v1/computers/:persona/control", post(computer_control))
        .route("/v1/computers/:persona/open", post(computer_open))
        .route("/v1/computers/:persona/secret", post(computer_secret))
        .route("/v1/computers/:persona/frames/:id", get(computer_frame))
        .route("/v1/sessions/:id/grants", post(session_grant))
        .route("/v1/route", post(route_message))
        .route("/v1/agui/:persona", post(agui_run))
        .route("/ws/computer/:persona", get(computer_socket))
        .route("/v1/settings/computer", post(settings_set_computer))
        .route("/v1/settings/generative-ui", post(settings_set_generative_ui))
        .route("/v1/ledger", get(ledger_get))
        .route("/v1/egress", get(egress_list))
        .route("/v1/egress/clear", post(egress_clear))
        .route("/v1/egress/local-only", post(egress_set_local_only))
        .route("/v1/settings", get(settings_get))
        .route("/v1/settings/effort", post(settings_set_effort))
        .route("/v1/settings/approvals", post(settings_set_approvals))
        .route("/v1/settings/sessions-peek", post(settings_set_sessions_peek))
        .route("/v1/settings/context-bar", post(settings_set_context_bar))
        .route("/v1/settings/standing-instructions", post(settings_set_standing))
        .route("/v1/settings/stall", post(settings_set_stall))
        .route("/v1/settings/audit-retention", post(settings_set_audit_retention))
        .route("/v1/settings/nav-layout", post(settings_set_nav_layout))
        .route("/v1/settings/scratch-base", post(settings_set_scratch_base))
        .route("/v1/settings/onboarded", post(settings_set_onboarded))
        .route("/v1/voice/devices", get(voice_devices))
        .route("/v1/voice/device", post(voice_set_device))
        .route("/v1/usage/rollup", get(usage_rollup))
        .route("/v1/telegram", get(telegram_get).post(telegram_set))
        .route("/v1/email-bot", get(email_bot_get).post(email_bot_set))
        .route("/v1/expenses", get(expenses_get).post(expenses_set))
        .route("/v1/expenses/routine", post(expenses_routine))
        .route("/v1/chief-of-staff", get(chief_get).post(chief_set))
        .route("/v1/chief-of-staff/routine", post(chief_routine))
        .route("/v1/bug-repro", get(repro_get).post(repro_set))
        .route("/v1/perf", get(perf_get).post(perf_set))
        .route("/v1/perf/check", post(perf_check))
        .route("/v1/perf/routine", post(perf_routine))
        .route("/v1/settings/default-model", post(settings_set_default_model))
        .route("/v1/settings/small-model", post(settings_set_small_model))
        .route("/v1/settings/embed-model", post(settings_set_embed_model))
        .merge(provider_routes())
        .route("/v1/providers/verify", post(provider_verify))
        .route("/v1/cloud/status", get(cloud_status))
        .route("/ws/events", get(events_socket))
        .route("/ws/session/:id", get(session_socket))
        .fallback(not_implemented)
        .layer(guard)
        // The GUI is served from the Vite/webview origin, so its calls to this
        // server are cross-origin. Loopback-only + a required token remain the
        // actual access control; CORS just has to not block the browser.
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::any())
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(ctx);

    // Loopback only — this must never be reachable off the machine.
    let (listener, fell_back) = bind_loopback(port).await?;
    let bound = listener.port();

    // Dev builds write the launch token to a user-only file so a separately-run GUI dev
    // server can authenticate. Release builds keep it in memory and hand it to the window
    // directly — it never touches disk.
    //
    // Named for the port we ACTUALLY bound, and written only AFTER binding. It used to be
    // a literal "sidecar-8765.token" written before the bind, which meant a run that never
    // got the port still left a file claiming to be the credentials for 8765 — a stale
    // token pointing at somebody else's server. That cost real debugging time.
    #[cfg(debug_assertions)]
    {
        let path = token_dir.join(format!("sidecar-{bound}.token"));
        if std::fs::write(&path, &token).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
        }
    }
    let (tx, ended_rx) = serve_both(listener, router);
    Ok(RunningApi { port: bound, token, fell_back, shutdown: Some(tx), ended: Some(ended_rx) })
}
