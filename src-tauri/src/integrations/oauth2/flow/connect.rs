use super::super::*;

/// Run the authorization-code + PKCE flow for a connector, store its tokens, and
/// return a human identity label.
pub async fn connect(app: tauri::AppHandle, spec: &OAuthSpec) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;
    let (client_id, client_secret) = get_client(spec.id).ok_or_else(|| {
        format!("{} needs an app registration first: create one with the vendor and paste its client id (and secret) in Integrations.", spec.name)
    })?;
    if spec.needs_secret && client_secret.trim().is_empty() {
        return Err(format!("{} requires a client secret from its app registration.", spec.name));
    }

    let verifier = b64url(&rand::random::<[u8; 32]>());
    let challenge = {
        use sha2::{Digest, Sha256};
        b64url(&Sha256::digest(verifier.as_bytes()))
    };
    let state = b64url(&rand::random::<[u8; 16]>());

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Could not open a local port: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let mut auth = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={challenge}&code_challenge_method=S256",
        spec.auth_url,
        urlencode(&client_id),
        urlencode(&redirect),
        urlencode(spec.scopes),
        urlencode(&state),
    );
    for (k, v) in spec.extra_auth {
        auth.push_str(&format!("&{k}={}", urlencode(v)));
    }
    app.shell().open(&auth, None).map_err(|e| format!("Could not open the browser: {e}"))?;

    let (code, callback_query) = tokio::task::spawn_blocking(move || await_code(listener))
        .await
        .map_err(|e| format!("Loopback task failed: {e}"))??;

    let mut form: Vec<(&str, String)> = vec![
        ("grant_type", "authorization_code".into()),
        ("code", code),
        ("redirect_uri", redirect.clone()),
        ("client_id", client_id.clone()),
        ("code_verifier", verifier),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret));
    }
    let resp = crate::core::egress::client("oauth")
        .post(spec.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| format!("Token response wasn't JSON: {e}"))?;
    if !status.is_success() || body["access_token"].as_str().unwrap_or("").is_empty() {
        let msg = body["error_description"].as_str().or_else(|| body["error"].as_str()).unwrap_or("unknown error");
        return Err(format!("{} rejected the authorization: {msg}", spec.name));
    }
    save_tokens(spec.id, &body);
    discover_instance(spec, &body, &callback_query).await;

    let identity = identity(spec).await.unwrap_or_else(|_| format!("{} account", spec.name));
    secrets::set_account(spec.id, &identity);
    Ok(identity)
}

/// Some vendors don't hand you a usable API base with the token: QuickBooks puts
/// the company id (`realmId`) in the callback query, and Docusign needs a
/// userinfo call to learn the account's base uri. Both are resolved once at
/// connect time and stored as this connector's `{instance}`.
pub(in crate::integrations::oauth2) async fn discover_instance(spec: &OAuthSpec, token_body: &Value, callback_query: &str) {
    let instance = match spec.id {
        "quickbooks" => param(callback_query, "realmId"),
        "docusign" => docusign_base(token_body["access_token"].as_str().unwrap_or("")).await,
        _ => None,
    };
    if let Some(v) = instance {
        if let Some(mut tokens) = load_tokens(spec.id) {
            tokens["instance_url"] = Value::String(v);
            secrets::set_integration_token(spec.id, &tokens.to_string());
        }
    }
}

/// Docusign's per-account API base (`{base_uri}/restapi/v2.1/accounts/{id}`).
pub(in crate::integrations::oauth2) async fn docusign_base(access: &str) -> Option<String> {
    let info: Value = crate::core::egress::client("oauth")
        .get("https://account-d.docusign.com/oauth/userinfo")
        .bearer_auth(access)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let acct = info["accounts"].as_array()?.first()?;
    Some(format!("{}/restapi/v2.1/accounts/{}", acct["base_uri"].as_str()?, acct["account_id"].as_str()?))
}

/// A valid access token, refreshing it when it's expired and we hold a refresh token.
pub(in crate::integrations::oauth2) async fn access_token(spec: &OAuthSpec) -> Result<String, String> {
    let tokens = load_tokens(spec.id).ok_or_else(|| format!("{} isn't connected.", spec.name))?;
    let obtained = tokens["obtained_at"].as_u64().unwrap_or(0);
    let expires = tokens["expires_in"].as_u64().unwrap_or(0);
    let fresh = expires == 0 || now() < obtained + expires.saturating_sub(60);
    if fresh {
        return Ok(tokens["access_token"].as_str().unwrap_or("").to_string());
    }
    let refresh = tokens["refresh_token"].as_str().unwrap_or("");
    if refresh.is_empty() {
        return Err(format!("{}'s access expired — reconnect it in Integrations.", spec.name));
    }
    let (client_id, client_secret) = get_client(spec.id).ok_or("Missing app registration.")?;
    let mut form: Vec<(&str, String)> = vec![
        ("grant_type", "refresh_token".into()),
        ("refresh_token", refresh.to_string()),
        ("client_id", client_id),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret));
    }
    let body: Value = crate::core::egress::client("oauth")
        .post(spec.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Refresh failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if body["access_token"].as_str().unwrap_or("").is_empty() {
        return Err(format!("{} refused to refresh access — reconnect it.", spec.name));
    }
    // Keep the existing refresh token when the vendor doesn't send a new one.
    let mut merged = body.clone();
    if merged["refresh_token"].as_str().is_none() {
        merged["refresh_token"] = Value::String(refresh.to_string());
    }
    save_tokens(spec.id, &merged);
    Ok(merged["access_token"].as_str().unwrap_or("").to_string())
}

pub(in crate::integrations::oauth2) fn api_base(spec: &OAuthSpec) -> String {
    let instance = load_tokens(spec.id)
        .and_then(|t| t["instance_url"].as_str().map(String::from))
        .unwrap_or_default();
    spec.api_base.replace("{instance}", &instance)
}

pub(in crate::integrations::oauth2) async fn get(spec: &OAuthSpec, token: &str, path: &str) -> Result<Value, String> {
    fetch(spec, token, path, None).await
}

/// GET, or POST with a JSON body when the feed declares one.
pub(in crate::integrations::oauth2) async fn fetch(spec: &OAuthSpec, token: &str, path: &str, post_body: Option<&str>) -> Result<Value, String> {
    let url = format!("{}{}", api_base(spec).trim_end_matches('/'), path);
    let client = crate::core::egress::client("oauth");
    let req = match post_body {
        Some(body) => client.post(&url).header("Content-Type", "application/json").body(body.to_string()),
        None => client.get(&url),
    };
    let resp = req
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error reaching {}: {e}", spec.name))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    match status.as_u16() {
        200..=299 => serde_json::from_str(&text).map_err(|e| format!("{} returned unreadable JSON: {e}", spec.name)),
        401 | 403 => Err(format!("{} refused the request — reconnect it in Integrations.", spec.name)),
        s => Err(format!("{} returned status {s}: {}", spec.name, text.chars().take(160).collect::<String>())),
    }
}

/// The connected account's label.
pub async fn identity(spec: &OAuthSpec) -> Result<String, String> {
    if spec.verify_path.is_empty() {
        return Ok(format!("{} account", spec.name));
    }
    let token = access_token(spec).await?;
    let v = get(spec, &token, spec.verify_path).await?;
    Ok(crate::integrations::rest::dig_path(&v, spec.identity_at)
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| format!("{} account", spec.name)))
}
