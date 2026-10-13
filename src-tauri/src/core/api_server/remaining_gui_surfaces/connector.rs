use super::super::*;

/// `POST /v1/connectors/:name/disallow` — stop serving this sender.
pub(in crate::core::api_server) async fn connector_disallow(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let user_id = body["user_id"].as_str().unwrap_or("").trim().to_string();
    if user_id.is_empty() {
        return Json(json!({ "ok": false, "error": "which sender?" }));
    }
    if name != "telegram" {
        return Json(json!({ "ok": false, "error": format!("{name} has no per-sender allowlist in this app") }));
    }
    let mut settings = crate::core::settings::Settings::load(&dir);
    match crate::integrations::telegram::disallow(&mut settings, &user_id) {
        Ok(changed) => {
            if changed {
                settings.save(&dir);
                crate::core::audit::log(&dir, "connector", "user", &name, &format!("disallowed {user_id}"), true);
            }
            Json(json!({ "ok": true, "changed": changed, "allowed_users": allowed_users(&name, &settings) }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// `POST /v1/connectors/:name/unauthorized/:id` — decide about one parked message.
///
/// `dismiss` forgets it. `allow` adds the sender to the allowlist and forgets the
/// message. `allow_deliver` does both and answers the message that was refused.
///
/// The third one is the only path by which text from an unvouched-for sender
/// reaches the model, and it takes a human clicking a button next to that text on
/// screen. It grants nothing the allowlist entry does not already grant a second
/// later — see the note in `core::unrouted`.
pub(in crate::core::api_server) async fn connector_unauthorized_resolve(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path((name, id)): axum::extract::Path<(String, String)>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let action = body["action"].as_str().unwrap_or("dismiss");
    if !matches!(action, "dismiss" | "allow" | "allow_deliver") {
        return Json(json!({ "ok": false, "error": format!("{action:?} is not one of dismiss, allow, allow_deliver") }));
    }
    let store = crate::core::unrouted::Unrouted::load(&dir);
    // Taken first, and once: two clicks must not answer the same message twice.
    let Some(item) = store.take(&id) else {
        return Json(json!({ "ok": false, "error": "that message is no longer in the queue" }));
    };
    if action == "dismiss" {
        return Json(json!({ "ok": true, "action": "dismiss" }));
    }
    if name != "telegram" {
        return Json(json!({ "ok": false, "error": format!("{name} has no per-sender allowlist in this app") }));
    }

    let mut settings = crate::core::settings::Settings::load(&dir);
    if crate::integrations::telegram::allow(&mut settings, &item.chat_id) {
        settings.save(&dir);
        crate::core::audit::log(&dir, "connector", "user", &name, &format!("allowed {}", item.chat_id), true);
    }
    if action == "allow" {
        return Json(json!({ "ok": true, "action": "allow", "allowed_users": allowed_users(&name, &settings) }));
    }

    // Answered through the same function the poll loop uses, so a delivered message
    // lands in the same session, with the same history, and gets its reply on the
    // same channel as one that arrived normally.
    let model = settings.ollama_model.clone();
    match crate::answer_telegram(&ctx.app, &dir, &model, &item.chat_id, &item.sender, &item.text).await {
        Ok(()) => Json(json!({ "ok": true, "action": "allow_deliver", "allowed_users": allowed_users(&name, &settings) })),
        // The allowlist entry stands either way — the user's decision about the
        // sender is not undone by a failure to send one reply.
        Err(e) => Json(json!({ "ok": false, "error": format!("allowed them, but the reply didn't send: {e}") })),
    }
}

/// Connect a connector with the fields its descriptor declared. Routes to the
/// right mechanism per connector: bespoke verifiers, the REST registry, IMAP, or
/// an OAuth app registration (which then needs the browser handshake).
pub(in crate::core::api_server) async fn connector_connect_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let Some(desc) = crate::integrations::descriptors::find(&name) else {
        return Json(json!({ "ok": false, "error": format!("unknown connector: {name}") }));
    };
    if !desc.implemented {
        return Json(json!({ "ok": false, "error": format!("{} isn't wired up in this build yet.", desc.title) }));
    }
    let fields = body["fields"].clone();
    let get = |k: &str| fields[k].as_str().unwrap_or("").trim().to_string();

    let result: Result<String, String> = match name.as_str() {
        "slack" => {
            let t = get("bot_token");
            if t.is_empty() { Err("paste the bot token (xoxb-…)".into()) } else { crate::integrations::slack::verify(&t).await.map(|id| { crate::core::secrets::set_integration_token("slack", &t); id }) }
        }
        // Several ways in, checked against the live API; kept whole, as one connection.
        "prometheus" => match crate::integrations::prometheus::Conn::from_fields(&fields) {
            Ok(conn) => conn.verify().await.map(|label| {
                crate::core::secrets::set_integration_token(crate::integrations::prometheus::ID, &conn.to_stored());
                label
            }),
            Err(e) => Err(e),
        },
        "email" => {
            let creds = crate::integrations::email::MailCreds {
                address: get("address"),
                password: get("app_password"),
                imap_host: get("imap_host"),
                smtp_host: get("smtp_host"),
            };
            if creds.address.is_empty() || creds.password.is_empty() || creds.imap_host.is_empty() {
                Err("address, app password and IMAP host are all required".into())
            } else {
                let stored = json!({ "address": creds.address, "app_password": creds.password, "imap_host": creds.imap_host, "smtp_host": creds.smtp_host }).to_string();
                // IMAP login blocks; keep it off the async workers.
                tokio::task::spawn_blocking(move || crate::integrations::email::verify_creds(&creds))
                    .await
                    .map_err(|e| format!("verify task failed: {e}"))
                    .and_then(|r| r)
                    .map(|id| { crate::core::secrets::set_integration_token("email", &stored); id })
            }
        }
        // OAuth connectors: store the app registration, then run the browser flow.
        _ if crate::integrations::oauth2::find_spec(&name).is_some() => {
            let spec = crate::integrations::oauth2::find_spec(&name).unwrap();
            let (cid, csec) = (get("client_id"), get("client_secret"));
            if !cid.is_empty() {
                crate::integrations::oauth2::set_client(&name, &cid, &csec);
            }
            if !crate::integrations::oauth2::has_client(&name) {
                Err(format!("{} needs an app registration first — create one with the vendor and paste its client id (and secret).", desc.title))
            } else {
                crate::integrations::oauth2::connect(ctx.app.clone(), spec).await
            }
        }
        // REST-registry connectors: verify the credential against the live API.
        _ if crate::integrations::rest::find_spec(&name).is_some() => {
            let stored = match (desc.fields.len(), fields.as_object()) {
                (1, Some(map)) => map.values().next().and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
                (_, Some(_)) => fields.to_string(),
                _ => String::new(),
            };
            if stored.is_empty() {
                Err("the credential is empty".into())
            } else {
                crate::integrations::verify_rest(&name, &stored)
                    .await
                    .map(|id| { crate::core::secrets::set_integration_token(&name, &stored); id })
            }
        }
        // Bespoke token integrations (GitHub, Notion, Linear, Stripe, Telegram).
        _ => {
            let token = fields.as_object().and_then(|m| m.values().next()).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if token.is_empty() {
                Err("the credential is empty".into())
            } else {
                crate::integrations::connect_token(&name, &token).await
            }
        }
    };

    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    match result {
        Ok(account) => {
            crate::core::secrets::set_account(&name, &account);
            crate::core::audit::log(&dir, "connect", "user", &name, &account, true);
            Json(json!({ "ok": true, "account": account }))
        }
        Err(e) => {
            crate::core::audit::log(&dir, "connect", "user", &name, &e, false);
            Json(json!({ "ok": false, "error": e }))
        }
    }
}

pub(in crate::core::api_server) async fn connector_disconnect_api(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let ok = crate::core::secrets::clear_integration_token(crate::integrations::token_key(&name));
    crate::core::audit::log(&dir, "connect", "user", &name, "disconnected", ok);
    Json(json!({ "ok": ok }))
}
