//! recording rules

use super::*;

/// Credential-shaped substrings, each with what replaces it.
pub(super) fn credential_patterns() -> &'static [(regex::Regex, &'static str)] {
    static PATTERNS: OnceLock<Vec<(regex::Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            (r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(-----END [A-Z ]*PRIVATE KEY-----|$)", "[private key]"),
            (r"(?i)\b(bearer|basic)\s+[A-Za-z0-9._~+/=-]{8,}", "$1 [credential]"),
            (r"\bsk-(ant-)?[A-Za-z0-9_-]{16,}", "[credential]"),
            (r"\bgh[pousr]_[A-Za-z0-9]{20,}", "[credential]"),
            (r"\bgithub_pat_[A-Za-z0-9_]{20,}", "[credential]"),
            (r"\b(AKIA|ASIA)[0-9A-Z]{16}\b", "[credential]"),
            (r"\bxox[baprs]-[A-Za-z0-9-]{10,}", "[credential]"),
            (r"\bglpat-[A-Za-z0-9_-]{20,}", "[credential]"),
            (
                r"(?i)\b(password|passwd|pwd|secret|token|api[_-]?key|access[_-]?key|client[_-]?secret|auth)=([^\s&;]+)",
                "$1=[credential]",
            ),
            (r"(?i)://([^/\s:@]+):([^/\s@]+)@", "://$1:[credential]@"),
        ]
        .iter()
        .filter_map(|(p, r)| regex::Regex::new(p).ok().map(|re| (re, *r)))
        .collect()
    })
}

/// Replace anything shaped like a credential, keeping the words around it.
///
/// A command is recorded in full, and a command can carry a token
/// (`curl -H "Authorization: Bearer …"`). Keeping the command readable while
/// removing the token is the whole trick: the row still says what ran.
pub fn scrub(text: &str) -> String {
    let mut out = text.to_string();
    for (re, with) in credential_patterns() {
        if re.is_match(&out) {
            out = re.replace_all(&out, *with).into_owned();
        }
    }
    out
}

/// Keys whose values are never written down, whatever they hold.
pub(super) const SECRET_KEYS: &[&str] = &[
    "content", "text", "body", "prompt", "result", "tool_arguments", "tool_result", "token", "tokens", "secret",
    "secrets", "password", "passwd", "api_key", "apikey", "authorization", "credentials", "credential", "cookie",
    "access_token", "refresh_token", "client_secret", "private_key", "value",
];

/// A JSON payload with every value under a sensitive key replaced — redaction by
/// name, because a payload's shape is known and its contents
/// are not.
pub fn redact(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, val)| {
                    let key = k.to_ascii_lowercase().replace('-', "_");
                    if SECRET_KEYS.contains(&key.as_str()) {
                        (k.clone(), Value::String("[redacted]".into()))
                    } else {
                        (k.clone(), redact(val))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        Value::String(s) => Value::String(scrub(s)),
        other => other.clone(),
    }
}

/// A URL as the trail records it: scheme, host and path. The query and fragment
/// are where tokens travel, and the userinfo is a password by definition.
pub fn clean_url(url: &str) -> String {
    let url = url.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.split(['?', '#']).next().unwrap_or("").to_string();
    };
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host = authority.rsplit('@').next().unwrap_or(authority);
    format!("{scheme}://{host}{path}")
}
