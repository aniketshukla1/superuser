use super::*;

/// Limits, past which this fails CLOSED — an argument tree too big to inspect is an
/// argument tree that goes nowhere, because "we could not check" must never read as "it
/// is fine".
pub(super) const MAX_NODES: usize = 2_000;

pub(super) const MAX_DEPTH: usize = 20;

pub(super) const MAX_STRING: usize = 64 * 1024;

/// What was found, and where it was: a category and a path, never the value.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub kind: &'static str,
    /// The path to it in the arguments — `notes.body`, `rows[2].token`.
    pub at: String,
}

impl Finding {
    pub fn say(&self) -> String {
        format!("{} at `{}`", self.kind, self.at)
    }
}

/// The sentence a refused call hands back: what was found, where, and what to do instead.
pub fn refusal(server: &str, findings: &[Finding]) -> String {
    let what: Vec<String> = findings.iter().take(4).map(|f| f.say()).collect();
    format!(
        "Refused. That call to {server} carries {} — {}. It was not sent. Nothing of it has left this machine. \
         Take the credential out and send the part that is actually needed; if the app genuinely needs a secret, \
         the person sets that up in its connection rather than pasting it through a tool call.",
        if findings.len() == 1 { "a credential" } else { "credentials" },
        what.join("; ")
    )
}

/// Inspect a call's arguments. Empty means nothing of this kind was found.
pub fn inspect(args: &Value) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut budget = MAX_NODES;
    walk(args, String::new(), 0, &mut budget, &mut findings);
    if budget == 0 {
        // Fail closed: an argument tree this size cannot be vouched for.
        findings.push(Finding { kind: "arguments too large to check", at: "(the whole call)".into() });
    }
    findings.dedup_by(|a, b| a.kind == b.kind && a.at == b.at);
    findings
}

pub(super) fn walk(v: &Value, at: String, depth: usize, budget: &mut usize, out: &mut Vec<Finding>) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    if depth > MAX_DEPTH {
        out.push(Finding { kind: "arguments nested too deeply to check", at });
        return;
    }
    match v {
        Value::Object(map) => {
            for (key, inner) in map {
                let path = if at.is_empty() { key.clone() } else { format!("{at}.{key}") };
                // A key that NAMES a credential is a finding wherever its value came from:
                // `{"password": "hunter2"}` is the same leak as a pasted PEM block.
                if let Some(kind) = credential_key(key) {
                    if !inner.is_null() && inner.as_str().map(|s| !s.trim().is_empty()).unwrap_or(true) {
                        out.push(Finding { kind, at: path.clone() });
                    }
                }
                walk(inner, path, depth + 1, budget, out);
            }
        }
        Value::Array(items) => {
            for (i, inner) in items.iter().enumerate() {
                walk(inner, format!("{at}[{i}]"), depth + 1, budget, out);
            }
        }
        Value::String(text) => {
            if text.len() > MAX_STRING {
                out.push(Finding { kind: "a value too long to check", at });
                return;
            }
            if let Some(kind) = credential_text(text) {
                out.push(Finding { kind, at });
            }
        }
        _ => {}
    }
}

/// A field name that means a credential, normalised the way people write them
/// (`x-api-key`, `API_KEY`, `apiKey`).
pub(super) fn credential_key(key: &str) -> Option<&'static str> {
    let flat: String = key.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    let flat = flat.strip_prefix("x").unwrap_or(&flat).to_string();
    let named = |n: &str| flat == n || flat.ends_with(n);
    if named("password") || named("passwd") || named("passphrase") {
        return Some("a password");
    }
    if named("apikey") || named("apisecret") || named("secretkey") || flat == "secret" || named("clientsecret") {
        return Some("an API key");
    }
    if named("accesstoken") || named("refreshtoken") || named("authtoken") || flat == "token" || named("bearertoken") {
        return Some("a token");
    }
    if named("privatekey") || named("sshkey") {
        return Some("a private key");
    }
    if named("authorization") {
        return Some("an authorization header");
    }
    None
}

/// A value whose SHAPE is a credential, whatever it is called.
pub(super) fn credential_text(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.contains("-----BEGIN") && t.contains("PRIVATE KEY") {
        return Some("a private key");
    }
    let lower = t.to_lowercase();
    if lower.starts_with("bearer ") && t.len() > 16 {
        return Some("a bearer token");
    }
    if lower.starts_with("basic ") && t.len() > 12 {
        return Some("a basic-auth header");
    }
    // Provider tokens, by the prefixes their issuers publish.
    for word in t.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',') {
        let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
        if w.len() < 20 {
            continue;
        }
        let provider = w.starts_with("sk-")
            || w.starts_with("sk_live_")
            || w.starts_with("rk_live_")
            || w.starts_with("github_pat_")
            || (w.len() > 20 && ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"].iter().any(|p| w.starts_with(p)))
            || w.starts_with("AKIA")
            || ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"].iter().any(|p| w.starts_with(p))
            || w.starts_with("AIza");
        if provider {
            return Some("what looks like a provider token");
        }
    }
    None
}

/// Why this address cannot be a connected app's server, or `None` if it can.
///
/// A custom MCP server is a URL a person pastes, and everything this app holds is on the
/// other end of it. Each line of the floor is a way people have actually
/// been phished or have shot themselves in the foot:
///
/// * **https only** — a plaintext endpoint hands every call to anyone on the path.
/// * **No credentials in the URL** — `https://user:pass@host` puts them in the trail, in
///   the config file, and in every error message that quotes the address.
/// * **No token in the query or fragment** — the same, one layer along.
/// * **Nothing on this machine or its network** — an "MCP server" at `localhost` or
///   `169.254.169.254` is not a connected app, it is this machine's own services.
/// * **A real name** — a single-label host or a bare IP is not something a person can
///   check, and `.local`, `.internal` and `.svc` are a network's private business.
pub fn mcp_endpoint_problem(url: &str) -> Option<String> {
    endpoint_problem(url, &CONNECTED_APP)
}

/// What an address is for, so a refusal names the thing the person was setting up.
pub struct Addressee {
    /// "server", "agent" — as in "give the server's address".
    pub noun: &'static str,
    /// "A connected app's server" — the subject of "has to be https".
    pub subject: &'static str,
    /// "a connected app" — as in "rather than a connected app on the internet".
    pub kind: &'static str,
    /// Where a credential belongs instead of the URL.
    pub credential_home: &'static str,
    /// A name that shows the shape of a good address.
    pub example: &'static str,
}

pub const CONNECTED_APP: Addressee = Addressee {
    noun: "server",
    subject: "A connected app's server",
    kind: "a connected app",
    credential_home: "the server's own connection",
    example: "mcp.example.com",
};

pub const REMOTE_AGENT: Addressee = Addressee {
    noun: "agent",
    subject: "A remote agent",
    kind: "a remote agent",
    credential_home: "the agent's token field",
    example: "agent.example.com",
};
