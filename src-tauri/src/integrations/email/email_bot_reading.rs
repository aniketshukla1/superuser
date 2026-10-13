//! the email bot's reading (see `core::email_bot`)

use super::*;

/// One new message, as the email bot needs it: who sent it, what they wrote this time, what
/// a reply threads under, and what tells a person's mail from a machine's.
#[derive(Debug, Clone, Default)]
pub struct Inbound {
    pub uid: u32,
    pub message_id: String,
    /// As a person reads it: `Ada Lovelace <ada@example.com>`.
    pub from: String,
    /// Lowercased: `ada@example.com`.
    pub from_addr: String,
    pub subject: String,
    /// The day it was sent, `YYYY-MM-DD` in the sender's own offset; empty if the header
    /// didn't parse.
    pub date: String,
    pub references: String,
    /// What they wrote this time: quoted history cut, capped.
    pub text: String,
    pub auto_submitted: String,
    pub precedence: String,
    /// Sent through a list: `List-Id` or `List-Unsubscribe` present.
    pub list_mail: bool,
    pub auth_results: String,
    pub return_path: String,
    /// `X-Autoreply` / `X-Autorespond` present.
    pub auto_reply_header: bool,
}

impl Inbound {
    pub fn from_raw(uid: u32, raw: &[u8]) -> Self {
        let p = crate::integrations::mail::parse(raw);
        let from = p.decoded("from");
        let lower = |name: &str| p.header(name).unwrap_or("").trim().to_ascii_lowercase();
        Inbound {
            uid,
            message_id: p.header("message-id").unwrap_or("").trim().to_string(),
            from_addr: crate::integrations::mail::address(&from),
            from,
            subject: p.decoded("subject"),
            date: chrono::DateTime::parse_from_rfc2822(p.header("date").unwrap_or("").trim())
                .map(|d| d.date_naive().format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            references: p.header("references").unwrap_or("").trim().to_string(),
            text: crate::integrations::mail::strip_quoted(&p.text),
            auto_submitted: lower("auto-submitted"),
            precedence: lower("precedence"),
            list_mail: p.has("list-id") || p.has("list-unsubscribe"),
            auth_results: lower("authentication-results"),
            return_path: p.header("return-path").unwrap_or("").trim().to_string(),
            auto_reply_header: p.has("x-autoreply") || p.has("x-autorespond"),
        }
    }
}

/// How far the email bot has read, and how many drafts it has written today.
///
/// IMAP numbers a mailbox's messages by UID, and the numbers only mean something under the
/// mailbox's UIDVALIDITY: when that changes, the bot starts again from now.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotState {
    #[serde(default)]
    pub started: bool,
    #[serde(default)]
    pub uid_validity: u32,
    #[serde(default)]
    pub last_uid: u32,
    /// The local day `drafted` counts, `YYYY-MM-DD`.
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub drafted: u32,
    /// Receipts read on `day` (`core::expenses`).
    #[serde(default)]
    pub extracted: u32,
}

pub(super) fn state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("email-bot.json")
}

pub fn load_state(data_dir: &Path) -> BotState {
    std::fs::read_to_string(state_path(data_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_state(data_dir: &Path, state: &BotState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        // Atomic: a torn file reads back as a fresh bot, which starts from now rather than
        // replaying — the safe way to be wrong.
        let _ = crate::core::store::write_atomic(&state_path(data_dir), &json);
    }
}

/// The UID range to fetch next, or `None` to start from now: a bot that has never looked, or
/// a mailbox whose numbering changed, reads nothing that is already there. Answering a
/// backlog would reply to week-old mail — the Slack watcher did exactly that in a fresh
/// folder.
pub fn next_range(state: &BotState, uid_validity: u32) -> Option<String> {
    (state.started && state.uid_validity == uid_validity).then(|| format!("{}:*", state.last_uid + 1))
}

/// Drafts written on `today`.
pub fn drafted_on(state: &BotState, today: &str) -> u32 {
    if state.day == today {
        state.drafted
    } else {
        0
    }
}

/// Receipts read on `today`.
pub fn extracted_on(state: &BotState, today: &str) -> u32 {
    if state.day == today {
        state.extracted
    } else {
        0
    }
}

/// A new day starts both counts at zero.
pub(super) fn on_day(data_dir: &Path, today: &str) -> BotState {
    let mut state = load_state(data_dir);
    if state.day != today {
        state.day = today.to_string();
        state.drafted = 0;
        state.extracted = 0;
    }
    state
}

/// Count one more draft on `today`.
pub fn note_drafted(data_dir: &Path, today: &str) {
    let mut state = on_day(data_dir, today);
    state.drafted += 1;
    save_state(data_dir, &state);
}

/// Count one more receipt read on `today`.
pub fn note_extracted(data_dir: &Path, today: &str) {
    let mut state = on_day(data_dir, today);
    state.extracted += 1;
    save_state(data_dir, &state);
}

/// New mail since the bot last looked, oldest first, at most `max` — the rest waits for the
/// next look. Read with PEEK, so nothing is marked as read.
pub fn poll_new(data_dir: &Path, max: usize) -> Result<Vec<Inbound>, String> {
    let c = creds()?;
    let mut s = session(&c)?;
    let mailbox = s.select("INBOX").map_err(|e| format!("Couldn't open INBOX: {e}"))?;
    let validity = mailbox.uid_validity.unwrap_or(0);
    let mut state = load_state(data_dir);
    let Some(range) = next_range(&state, validity) else {
        let newest = match mailbox.uid_next {
            Some(next) if next > 0 => next - 1,
            _ => s.uid_search("ALL").map(|all| all.into_iter().max().unwrap_or(0)).unwrap_or(0),
        };
        state.started = true;
        state.uid_validity = validity;
        state.last_uid = newest;
        save_state(data_dir, &state);
        let _ = s.logout();
        return Ok(Vec::new());
    };
    let fetched = s.uid_fetch(&range, "(UID BODY.PEEK[])").map_err(|e| format!("Couldn't fetch new mail: {e}"))?;
    // `N:*` always returns the newest message even when it is older than N, so the UID is
    // checked rather than trusted.
    let mut mail: Vec<Inbound> = fetched
        .iter()
        .filter_map(|m| {
            let uid = m.uid?;
            (uid > state.last_uid).then(|| Inbound::from_raw(uid, m.body().unwrap_or(&[])))
        })
        .collect();
    mail.sort_by_key(|m| m.uid);
    mail.truncate(max);
    if let Some(last) = mail.last() {
        state.last_uid = last.uid;
        save_state(data_dir, &state);
    }
    let _ = s.logout();
    Ok(mail)
}
