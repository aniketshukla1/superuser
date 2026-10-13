use super::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct McpCreds {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_endpoint: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

pub(super) struct AuthMeta {
    pub(super) authorization_endpoint: String,
    pub(super) token_endpoint: String,
    pub(super) registration_endpoint: Option<String>,
    pub(super) scopes: Option<String>,
}

pub(super) fn key(id: &str) -> String { format!("mcp_{id}") }

pub fn has(id: &str) -> bool { secrets::get_integration_token(&key(id)).is_some() }

pub fn load(id: &str) -> Option<McpCreds> {
    serde_json::from_str(&secrets::get_integration_token(&key(id))?).ok()
}

pub(super) fn save(id: &str, creds: &McpCreds) {
    if let Ok(s) = serde_json::to_string(creds) {
        secrets::set_integration_token(&key(id), &s);
    }
}

pub fn forget(id: &str) -> bool { secrets::clear_integration_token(&key(id)) }

pub(super) fn b64url(b: &[u8]) -> String { base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b) }

pub(super) fn urlencode(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}

pub(super) fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => { if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; } else { out.push(b[i]); i += 1; } }
            b'+' => { out.push(b' '); i += 1; }
            c => { out.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(super) fn param(query: &str, key: &str) -> Option<String> {
    query.split(['?', '&']).find_map(|kv| { let (k, v) = kv.split_once('=')?; (k == key).then(|| urldecode(v)) })
}

pub(super) fn origin(url: &str) -> String {
    // scheme://host[:port]
    if let Some(rest) = url.split_once("://") {
        let host = rest.1.split('/').next().unwrap_or("");
        return format!("{}://{}", rest.0, host);
    }
    url.to_string()
}

pub(super) async fn get_json(client: &crate::core::egress::Client, url: &str) -> Option<Value> {
    let r = client.get(url).header("MCP-Protocol-Version", "2025-06-18").send().await.ok()?;
    if r.status().is_success() { r.json().await.ok() } else { None }
}

/// Discover the authorization server + endpoints for an MCP server.
pub(super) async fn discover(client: &crate::core::egress::Client, server_url: &str) -> Result<AuthMeta, String> {
    // 1. Unauthenticated initialize → expect 401 + resource metadata pointer.
    let probe = client
        .post(server_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2025-06-18")
        .body(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"Superuser","version":"0.1.0"}}}).to_string())
        .send()
        .await
        .map_err(|e| format!("Could not reach {server_url}: {e}"))?;

    let www = probe.headers().get("WWW-Authenticate").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let resource_meta_url = www
        .split(',')
        .find_map(|p| p.trim().strip_prefix("resource_metadata=").map(|s| s.trim_matches('"').to_string()))
        .unwrap_or_else(|| format!("{}/.well-known/oauth-protected-resource", origin(server_url)));

    // 2. Protected-resource metadata → authorization server.
    let as_url = get_json(client, &resource_meta_url)
        .await
        .and_then(|v| v["authorization_servers"][0].as_str().map(String::from))
        .unwrap_or_else(|| origin(server_url));

    // 3. Authorization-server metadata.
    let meta = match get_json(client, &format!("{}/.well-known/oauth-authorization-server", as_url.trim_end_matches('/'))).await {
        Some(v) => v,
        None => get_json(client, &format!("{}/.well-known/openid-configuration", as_url.trim_end_matches('/')))
            .await
            .ok_or("Could not find the authorization server metadata for this MCP server.")?,
    };

    Ok(AuthMeta {
        authorization_endpoint: meta["authorization_endpoint"].as_str().ok_or("No authorization_endpoint")?.to_string(),
        token_endpoint: meta["token_endpoint"].as_str().ok_or("No token_endpoint")?.to_string(),
        registration_endpoint: meta["registration_endpoint"].as_str().map(String::from),
        scopes: meta["scopes_supported"].as_array().map(|a| a.iter().filter_map(|s| s.as_str()).collect::<Vec<_>>().join(" ")),
    })
}

/// Dynamic Client Registration (RFC 7591). Returns (client_id, client_secret?).
pub(super) async fn register(client: &crate::core::egress::Client, endpoint: &str, redirect: &str) -> Result<(String, Option<String>), String> {
    let v: Value = client
        .post(endpoint)
        .json(&json!({
            "client_name": "Superuser",
            "redirect_uris": [redirect],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
            "application_type": "native"
        }))
        .send()
        .await
        .map_err(|e| format!("Client registration failed: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let cid = v["client_id"].as_str().ok_or("Registration did not return a client_id")?.to_string();
    Ok((cid, v["client_secret"].as_str().map(String::from)))
}
