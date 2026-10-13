use super::*;

pub(super) const SCOPES: &str = "openid email https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/calendar.readonly";

#[derive(Serialize, Deserialize)]
pub(super) struct GoogleCreds {
    pub(super) client_id: String,
    pub(super) client_secret: String,
    pub(super) refresh_token: String,
}

#[derive(Serialize, Deserialize)]
pub(super) struct ClientCfg {
    pub(super) client_id: String,
    pub(super) client_secret: String,
}

/// Store the app-level Google OAuth client once; reused for every Google sign-in.
pub fn set_client(client_id: &str, client_secret: &str) {
    let cfg = serde_json::to_string(&ClientCfg { client_id: client_id.into(), client_secret: client_secret.into() }).unwrap_or_default();
    secrets::set_integration_token("google_client", &cfg);
}

pub fn has_client() -> bool {
    get_client().is_some()
}

/// A client baked into the build enables true one-click SSO with zero user
/// setup. Embed by building with BBA_GOOGLE_CLIENT_ID / BBA_GOOGLE_CLIENT_SECRET
/// set (Google "Desktop app" clients use PKCE, so the secret isn't confidential).
pub(super) fn default_client() -> Option<(String, String)> {
    match (option_env!("BBA_GOOGLE_CLIENT_ID"), option_env!("BBA_GOOGLE_CLIENT_SECRET")) {
        (Some(id), Some(sec)) if !id.is_empty() && !sec.is_empty() => Some((id.to_string(), sec.to_string())),
        _ => None,
    }
}

pub(super) fn get_client() -> Option<(String, String)> {
    // A user-configured client wins; otherwise fall back to a build-time default.
    if let Some(raw) = secrets::get_integration_token("google_client") {
        if let Ok(cfg) = serde_json::from_str::<ClientCfg>(&raw) {
            if !cfg.client_id.is_empty() && !cfg.client_secret.is_empty() {
                return Some((cfg.client_id, cfg.client_secret));
            }
        }
    }
    default_client()
}

pub(super) fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Run the full OAuth flow using the stored app-level client. Opens the browser,
/// catches the loopback redirect, exchanges the code, stores the refresh token,
/// and returns the signed-in email.
pub async fn oauth_connect(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;
    let (client_id, client_secret) = get_client().ok_or("Set up Google sign-in first (one-time): add an OAuth client from Google Cloud Console.")?;

    // PKCE verifier + challenge
    let verifier: String = {
        let mut rng = rand::thread_rng();
        (0..64).map(|_| {
            const CS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
            CS[rng.gen_range(0..CS.len())] as char
        }).collect()
    };
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));

    // Loopback listener on a random free port
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Could not open a local port: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        urlencode(&client_id), urlencode(&redirect), urlencode(SCOPES), challenge
    );
    app.shell().open(&auth_url, None).map_err(|e| format!("Could not open the browser: {e}"))?;

    // Wait (blocking thread) for Google to redirect back with ?code=
    let code = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        listener.set_nonblocking(false).ok();
        let (mut stream, _) = listener.accept().map_err(|e| format!("Loopback accept failed: {e}"))?;
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
        let req = String::from_utf8_lossy(&buf[..n]);
        let line = req.lines().next().unwrap_or("");
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body style=\"font-family:sans-serif;text-align:center;padding-top:80px\"><h2>\xE2\x9C\x85 Connected</h2><p>You can close this window and return to Superuser.</p></body></html>");
        // GET /?code=XXX&scope=... HTTP/1.1
        let q = line.split_whitespace().nth(1).unwrap_or("");
        if let Some(err) = param(q, "error") {
            return Err(format!("Google returned: {err}"));
        }
        param(q, "code").ok_or("No authorization code in the redirect.".to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Exchange code for tokens
    let client = crate::core::egress::client("google");
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Token exchange failed: {e}"))?;
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let refresh = v["refresh_token"].as_str().ok_or_else(|| format!("Google did not return a refresh token: {}", v["error_description"].as_str().unwrap_or("unknown error")))?;

    let creds = GoogleCreds { client_id, client_secret, refresh_token: refresh.to_string() };
    let json = serde_json::to_string(&creds).map_err(|e| e.to_string())?;
    secrets::set_integration_token("google", &json);

    // Fetch the signed-in email so the UI can show which account is connected.
    let mut email = "your Google account".to_string();
    if let Some(at) = v["access_token"].as_str() {
        if let Ok(info) = client.get("https://www.googleapis.com/oauth2/v3/userinfo").bearer_auth(at).send().await {
            if let Ok(j) = info.json::<Value>().await {
                if let Some(e) = j["email"].as_str() { email = e.to_string(); }
            }
        }
    }
    secrets::set_account("google", &email);
    Ok(email)
}

pub(super) fn param(query: &str, key: &str) -> Option<String> {
    query.split(['?', '&']).find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| urldecode(v))
    })
}

pub(super) fn urlencode(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}

pub(super) fn urldecode(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(b); i += 3; } else { out.push(bytes[i]); i += 1; }
            }
            b'+' => { out.push(b' '); i += 1; }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Get a fresh access token from the stored refresh token.
pub(super) async fn access_token() -> Result<String, String> {
    let raw = secrets::get_integration_token("google").ok_or("Connect Google first.")?;
    let creds: GoogleCreds = serde_json::from_str(&raw).map_err(|_| "Stored Google credentials are corrupted — reconnect.")?;
    let client = crate::core::egress::client("google");
    let v: Value = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("refresh_token", creds.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Google token refresh failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    v["access_token"].as_str().map(String::from).ok_or_else(|| format!("Google refused the refresh token: {}", v["error_description"].as_str().unwrap_or("reconnect Google")))
}

pub(super) async fn gapi(client: &crate::core::egress::Client, token: &str, url: &str) -> Result<Value, String> {
    let resp = client.get(url).bearer_auth(token).send().await.map_err(|e| format!("Network error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Google API returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}
