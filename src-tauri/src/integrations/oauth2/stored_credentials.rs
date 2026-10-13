//! stored credentials

use super::*;

pub(super) fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(super) fn client_key(id: &str) -> String {
    format!("oauth_client_{id}")
}

/// Store the app registration the user created with the vendor.
pub fn set_client(id: &str, client_id: &str, client_secret: &str) {
    let v = serde_json::json!({ "client_id": client_id, "client_secret": client_secret });
    secrets::set_integration_token(&client_key(id), &v.to_string());
}

pub fn has_client(id: &str) -> bool {
    secrets::get_integration_token(&client_key(id)).is_some()
}

pub(super) fn get_client(id: &str) -> Option<(String, String)> {
    let raw = secrets::get_integration_token(&client_key(id))?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    Some((v["client_id"].as_str()?.to_string(), v["client_secret"].as_str().unwrap_or("").to_string()))
}

pub(super) fn save_tokens(id: &str, body: &Value) {
    let mut stored = serde_json::json!({
        "access_token": body["access_token"].as_str().unwrap_or(""),
        "obtained_at": now(),
    });
    if let Some(r) = body["refresh_token"].as_str() {
        stored["refresh_token"] = Value::String(r.to_string());
    }
    if let Some(e) = body["expires_in"].as_u64() {
        stored["expires_in"] = Value::from(e);
    }
    // Salesforce returns the org's API host with the token.
    if let Some(i) = body["instance_url"].as_str() {
        stored["instance_url"] = Value::String(i.to_string());
    }
    secrets::set_integration_token(id, &stored.to_string());
}

pub(super) fn load_tokens(id: &str) -> Option<Value> {
    serde_json::from_str(&secrets::get_integration_token(id)?).ok()
}
