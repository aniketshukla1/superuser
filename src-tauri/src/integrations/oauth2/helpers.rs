//! helpers

use super::*;

pub(super) fn b64url(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub(super) fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.replace('+', " "))
    })
}

/// Block until the vendor redirects back to our loopback port, then return the
/// code **and the full callback query** (some vendors ride extra context there —
/// Intuit returns the QuickBooks `realmId` this way, not in the token response).
pub(super) fn await_code(listener: TcpListener) -> Result<(String, String), String> {
    let (mut stream, _) = listener.accept().map_err(|e| format!("Loopback accept failed: {e}"))?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).map_err(|e| format!("Loopback read failed: {e}"))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let line = req.lines().next().unwrap_or("");
    let query = line.split_whitespace().nth(1).unwrap_or("").split_once('?').map(|(_, q)| q.to_string()).unwrap_or_default();
    let body = "<html><body style=\"font-family:system-ui;padding:40px\"><h2>Connected ✓</h2><p>You can close this tab and return to Superuser.</p></body></html>";
    let _ = stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
    if let Some(err) = param(&query, "error") {
        return Err(format!("Authorization was refused: {err}"));
    }
    let code = param(&query, "code").ok_or_else(|| "No authorization code in the redirect.".to_string())?;
    Ok((code, query))
}
