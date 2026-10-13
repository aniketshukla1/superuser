use super::*;

/// How the credential is attached to the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStyle {
    /// `Authorization: Bearer <token>`
    Bearer,
    /// `Authorization: <token>` (verbatim — Linear/Asana-style personal keys)
    Raw,
    /// `Authorization: Basic base64(<email>:<token>)` — Atlassian, Zendesk
    BasicEmailToken,
    /// A custom header + value prefix, e.g. `("X-Figma-Token", "")` or
    /// `("Authorization", "Token token=")` for PagerDuty.
    Header(&'static str, &'static str),
    /// The credential rides in a query parameter, e.g. Hunter's `?api_key=`.
    Query(&'static str),
}

/// One list endpoint to pull, and how to turn its items into memories.
#[derive(Debug, Clone, Copy)]
pub struct Feed {
    /// Path appended to the base url (may contain `{site}`).
    pub path: &'static str,
    /// Dotted path to the array of items in the response (empty = the root array).
    pub items_at: &'static str,
    /// Field holding a stable id.
    pub id_key: &'static str,
    /// Fields tried in order for the item's title.
    pub title_keys: &'static [&'static str],
    /// Fields rendered into the body (label:field pairs).
    pub body_keys: &'static [(&'static str, &'static str)],
    /// A short noun for this feed, e.g. "issue" (becomes a tag).
    pub kind: &'static str,
    /// When set, the feed is fetched with POST and this JSON as the body
    /// (Dropbox-style APIs that expose listing only over POST).
    pub post_body: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct RestSpec {
    pub id: &'static str,
    pub name: &'static str,
    /// Base url; `{site}` is replaced by the stored `site`/`subdomain` field.
    pub base: &'static str,
    pub auth: AuthStyle,
    /// Endpoint used to confirm the credential works.
    pub verify_path: &'static str,
    /// Dotted path to a human identity in the verify response.
    pub identity_at: &'static str,
    pub feeds: &'static [Feed],
    /// Additional headers taken from credential fields: (header name, field key).
    /// Covers multi-key APIs like Datadog (DD-API-KEY + DD-APPLICATION-KEY).
    pub extra_headers: &'static [(&'static str, &'static str)],
}

/// Follow a dotted path (`a.b.c`) through a JSON value.
pub(super) fn dig<'a>(v: &'a Value, path: &str) -> &'a Value {
    let mut cur = v;
    if path.is_empty() {
        return cur;
    }
    for part in path.split('.') {
        cur = &cur[part];
    }
    cur
}

/// The credential for a connector: either a bare token or a field map.
pub struct Creds {
    pub(super) raw: String,
    pub(super) map: Option<Value>,
}

impl Creds {
    pub fn parse(stored: &str) -> Self {
        let map = serde_json::from_str::<Value>(stored).ok().filter(|v| v.is_object());
        Creds { raw: stored.to_string(), map }
    }
    pub fn field(&self, key: &str) -> Option<String> {
        self.map.as_ref()?.get(key)?.as_str().map(|s| s.to_string())
    }
    /// The secret itself (first of the usual token field names, else the raw value).
    pub fn token(&self) -> String {
        for k in ["token", "api_token", "api_key", "bot_token", "secret", "access_token"] {
            if let Some(v) = self.field(k) {
                return v;
            }
        }
        self.raw.clone()
    }
    pub(super) fn site(&self) -> String {
        self.field("site").or_else(|| self.field("subdomain")).unwrap_or_default()
    }
}

pub(super) fn apply_auth(req: crate::core::egress::RequestBuilder, spec: &RestSpec, creds: &Creds) -> crate::core::egress::RequestBuilder {
    let token = creds.token();
    let mut req = match spec.auth {
        AuthStyle::Bearer => req.header("Authorization", format!("Bearer {token}")),
        AuthStyle::Raw => req.header("Authorization", token),
        AuthStyle::Header(name, prefix) => req.header(name, format!("{prefix}{token}")),
        AuthStyle::Query(param) => req.query(&[(param, token.as_str())]),
        AuthStyle::BasicEmailToken => {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            let email = creds.field("email").unwrap_or_default();
            let enc = STANDARD.encode(format!("{email}:{token}"));
            req.header("Authorization", format!("Basic {enc}"))
        }
    };
    for (header, field) in spec.extra_headers {
        if let Some(v) = creds.field(field) {
            req = req.header(*header, v);
        }
    }
    req
}

pub(super) fn url_for(spec: &RestSpec, creds: &Creds, path: &str) -> String {
    let base = spec.base.replace("{site}", &creds.site());
    format!("{}{}", base.trim_end_matches('/'), path)
}

pub(super) async fn get(spec: &RestSpec, creds: &Creds, path: &str) -> Result<Value, String> {
    let client = crate::core::egress::client("connector");
    let url = url_for(spec, creds, path);
    let resp = apply_auth(client.get(&url), spec, creds)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error reaching {}: {e}", spec.name))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    match status.as_u16() {
        200..=299 => serde_json::from_str(&text).map_err(|e| format!("{} returned unreadable JSON: {e}", spec.name)),
        401 | 403 => Err(format!("{} rejected the credential — check it in Integrations.", spec.name)),
        404 => Err(format!("{} endpoint not found ({path}).", spec.name)),
        s => Err(format!("{} returned status {s}: {}", spec.name, text.chars().take(160).collect::<String>())),
    }
}

/// Confirm a credential works and return a human identity label.
pub async fn verify(spec: &RestSpec, stored: &str) -> Result<String, String> {
    let creds = Creds::parse(stored);
    let v = get(spec, &creds, spec.verify_path).await?;
    let who = dig(&v, spec.identity_at);
    Ok(who
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{} account", spec.name)))
}

pub(super) fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Object(_) => {
            // Common nested shapes: {name}, {title}, {display_name}, {content}
            for k in ["name", "title", "display_name", "content", "text", "email"] {
                if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
                    return s.to_string();
                }
            }
            String::new()
        }
        Value::Array(a) => a.iter().map(text_of).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
        Value::Null => String::new(),
    }
}
