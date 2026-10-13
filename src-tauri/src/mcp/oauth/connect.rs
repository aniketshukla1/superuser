use super::*;

/// Full connect flow. `manual_client_id` is used when the server doesn't support
/// dynamic registration.
pub async fn connect(app: tauri::AppHandle, id: &str, server_url: &str, manual_client_id: Option<String>) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    let client = crate::core::egress::client("mcp-auth");
    let meta = discover(&client, server_url).await?;

    // Loopback redirect
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Could not open a local port: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}/callback");

    let (client_id, client_secret) = match manual_client_id {
        Some(cid) if !cid.is_empty() => (cid, None),
        _ => match &meta.registration_endpoint {
            Some(ep) => register(&client, ep, &redirect).await?,
            None => return Err("This server requires a pre-registered OAuth client. Paste a client ID to continue.".into()),
        },
    };

    // PKCE
    let verifier: String = {
        let mut rng = rand::thread_rng();
        (0..64).map(|_| { const CS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~"; CS[rng.gen_range(0..CS.len())] as char }).collect()
    };
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    let state = b64url(&rand::thread_rng().gen::<[u8; 16]>());

    let mut auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        meta.authorization_endpoint, urlencode(&client_id), urlencode(&redirect), challenge, state
    );
    if let Some(scope) = &meta.scopes { auth_url.push_str(&format!("&scope={}", urlencode(scope))); }
    app.shell().open(&auth_url, None).map_err(|e| format!("Could not open the browser: {e}"))?;

    // Catch the redirect
    let code = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let (mut stream, _) = listener.accept().map_err(|e| format!("Loopback accept failed: {e}"))?;
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
        let req = String::from_utf8_lossy(&buf[..n]);
        let line = req.lines().next().unwrap_or("");
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body style=\"font-family:sans-serif;text-align:center;padding-top:80px\"><h2>\xE2\x9C\x85 Connected</h2><p>Return to Superuser.</p></body></html>");
        let q = line.split_whitespace().nth(1).unwrap_or("");
        if let Some(err) = param(q, "error") { return Err(format!("Authorization failed: {err}")); }
        param(q, "code").ok_or("No authorization code in the redirect.".into())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Exchange code → tokens
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code),
        ("redirect_uri", redirect),
        ("client_id", client_id.clone()),
        ("code_verifier", verifier),
    ];
    if let Some(sec) = &client_secret { form.push(("client_secret", sec.clone())); }
    let tok: Value = client.post(&meta.token_endpoint).form(&form).send().await
        .map_err(|e| format!("Token exchange failed: {e}"))?
        .json().await.map_err(|e| e.to_string())?;
    let access = tok["access_token"].as_str().ok_or_else(|| format!("No access token returned: {}", tok["error_description"].as_str().unwrap_or("unknown")))?;

    save(id, &McpCreds {
        access_token: access.to_string(),
        refresh_token: tok["refresh_token"].as_str().map(String::from),
        token_endpoint: meta.token_endpoint,
        client_id,
        client_secret,
    });
    Ok(())
}

/// Refresh the access token if we have a refresh token; returns a usable token.
pub async fn fresh_token(id: &str) -> Result<String, String> {
    let creds = load(id).ok_or("Not connected.")?;
    let Some(refresh) = creds.refresh_token.clone() else { return Ok(creds.access_token) };
    let client = crate::core::egress::client("mcp-auth");
    let mut form = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh),
        ("client_id", creds.client_id.clone()),
    ];
    if let Some(sec) = &creds.client_secret { form.push(("client_secret", sec.clone())); }
    let tok: Value = match client.post(&creds.token_endpoint).form(&form).send().await {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => return Ok(creds.access_token), // network blip — try the existing token
    };
    if let Some(new) = tok["access_token"].as_str() {
        let mut updated = creds.clone();
        updated.access_token = new.to_string();
        if let Some(rt) = tok["refresh_token"].as_str() { updated.refresh_token = Some(rt.to_string()); }
        save(id, &updated);
        Ok(new.to_string())
    } else {
        Ok(creds.access_token)
    }
}
