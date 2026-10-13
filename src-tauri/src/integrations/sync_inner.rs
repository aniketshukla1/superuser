use super::*;

/// Result of a real sync: how many NEW memories were created, plus their titles
/// (used by the background scheduler to push meaningful notifications).
#[derive(Debug, Default)]
pub struct SyncOutcome {
    pub created: u32,
    pub titles: Vec<String>,
}

/// Which keychain entry a given integration id uses (gmail + calendar share
/// the single Google connection).
pub fn token_key(id: &str) -> &str {
    match id {
        "gmail" | "calendar" | "googlecalendar" => "google",
        other => other,
    }
}

/// Normalize frontend ids to registry ids.
pub fn normalize_id(id: &str) -> &str {
    match id {
        "googlecalendar" => "calendar",
        other => other,
    }
}

/// Run a real sync for one integration id. Errors if not connected/unsupported.
///
/// A successful sync is snapshotted into the [ledger](crate::core::ledger) before
/// it returns, so "what changed since I last looked" is answerable afterwards. The
/// snapshot happens here rather than at each call site because there are several
/// (the scheduler, the agent's `connector_sync`, the API, the UI) and a source that
/// skipped it would silently have no history.
pub async fn sync_one(id: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let out = sync_inner(id, memory).await;
    if out.is_ok() {
        let source = normalize_id(id).to_string();
        // Best effort, always: a vault without git, or a repository in a state we
        // didn't expect, costs the delta — never the data that was just fetched.
        if let Err(e) = crate::core::ledger::Ledger::new(&memory.vault_path).snapshot(&source, "sync") {
            eprintln!("[superuser] ledger: snapshot after {source} sync skipped: {e}");
        }
    }
    out
}

pub(super) async fn sync_inner(id: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    match normalize_id(id) {
        "github" => {
            let token = secrets::get_integration_token("github").ok_or("Connect GitHub first — add a personal access token.")?;
            github::sync(&token, memory).await
        }
        "notion" => {
            let token = secrets::get_integration_token("notion").ok_or("Connect Notion first — add an internal integration token.")?;
            notion::sync(&token, memory).await
        }
        "gmail" => google::sync_gmail(memory).await,
        "calendar" => google::sync_calendar(memory).await,
        "linear" => {
            let token = secrets::get_integration_token("linear").ok_or("Connect Linear first — add an API key.")?;
            linear::sync(&token, memory).await
        }
        "stripe" => {
            let token = secrets::get_integration_token("stripe").ok_or("Connect Stripe first — add a secret key.")?;
            stripe::sync(&token, memory).await
        }
        "slack" => slack::sync(memory).await,
        // IMAP is blocking; keep it off the async runtime's worker threads.
        "email" => {
            let vault = memory.vault_path.clone();
            tokio::task::spawn_blocking(move || email::sync(&crate::core::memory::MemoryEngine::new(vault)))
                .await
                .map_err(|e| format!("email sync task failed: {e}"))?
        }
        // Asked live, when an assistant needs it: metrics a minute old are already history.
        "prometheus" => Err("Prometheus is queried live when an assistant needs it, so there's nothing to copy into memory — \
                             ask the Product Performance assistant, or use the metrics tools."
            .into()),
        // Outbound-only: the Cloud API delivers inbound over webhooks, which a
        // local-first desktop app doesn't host.
        "whatsapp" => Err("WhatsApp is send-only here — assistants can message it, but there's nothing to sync in.".into()),
        // Everything declared in the generic registries goes through one client.
        other => match (rest::find_spec(other), oauth2::find_spec(other)) {
            (Some(spec), _) => {
                // Resolves the default account's credential, falling back to a
                // single-account connection.
                let stored = accounts::credential(other, None)
                    .ok_or_else(|| format!("Connect {} first — add its credential in Integrations.", spec.name))?;
                rest::sync(spec, &stored, memory).await
            }
            (_, Some(spec)) => oauth2::sync(spec, memory).await,
            _ => Err(format!("The {other} integration isn't wired up yet.")),
        },
    }
}

/// The connectors with a bespoke arm in `sync_inner`. The generic registries are
/// read from their specs; these are the ones a spec cannot see.
pub(super) const BESPOKE_SYNC: &[&str] = &["github", "notion", "gmail", "calendar", "linear", "stripe", "slack", "email"];

/// Whether `sync_one` can copy this connector's content into memory — and so whether
/// `connector_sync`, `connector_search` and `connector_changes` have anything to work
/// on. The Connectors page lists those tools against exactly these connectors.
pub fn syncs(id: &str) -> bool {
    let id = normalize_id(id);
    BESPOKE_SYNC.contains(&id)
        || rest::find_spec(id).is_some()
        // Same bar as the catalog's `implemented`: an OAuth spec with no feeds reads nothing.
        || oauth2::find_spec(id).is_some_and(|s| !s.feeds.is_empty())
}

/// Verify + store a pasted-token connector, returning the account label. Shared by
/// the Tauri command and the GUI's connect endpoint so both behave identically.
pub async fn connect_token(id: &str, token: &str) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Paste a token first.".into());
    }
    let label = match id {
        "github" => github::verify(token).await?,
        "notion" => notion::verify(token).await?,
        "linear" => linear::verify(token).await?,
        "stripe" => stripe::verify(token).await?,
        "telegram" => {
            // getMe both validates the token and names the bot.
            let v: serde_json::Value = crate::core::egress::client("connector")
                .get(format!("https://api.telegram.org/bot{token}/getMe"))
                .send()
                .await
                .map_err(|e| format!("Couldn't reach Telegram: {e}"))?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            if !v["ok"].as_bool().unwrap_or(false) {
                return Err("Telegram rejected that bot token.".into());
            }
            format!("@{}", v["result"]["username"].as_str().unwrap_or("bot"))
        }
        other => return Err(format!("{other} doesn't use a pasted token.")),
    };
    let key = token_key(id);
    secrets::set_integration_token(key, token);
    secrets::set_account(key, &label);
    Ok(label)
}

/// Verify a credential for any REST-registry connector, returning an identity label.
pub async fn verify_rest(id: &str, stored: &str) -> Result<String, String> {
    let spec = rest::find_spec(id).ok_or_else(|| format!("{id} isn't a REST connector."))?;
    rest::verify(spec, stored).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub id: String,
    pub name: String,
    pub status: String, // "connected" | "disconnected" | "error"
    pub last_sync: Option<String>,
    pub memory_count: u32,
    /// Human label for the connected account, e.g. "@octocat" or an email.
    pub account: Option<String>,
}

pub struct Registry {
    pub(super) inner: Mutex<HashMap<String, Integration>>,
}

pub(super) const CATALOG: &[(&str, &str)] = &[
    ("github", "GitHub"),
    ("gmail", "Gmail"),
    ("slack", "Slack"),
    ("calendar", "Google Calendar"),
    ("notion", "Notion"),
    ("linear", "Linear"),
    ("stripe", "Stripe"),
];
