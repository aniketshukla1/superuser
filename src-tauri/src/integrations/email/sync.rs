use super::*;

pub struct MailCreds {
    pub address: String,
    pub password: String,
    pub imap_host: String,
    pub smtp_host: String,
}

/// Read the stored field map. `smtp_host` falls back to the IMAP host with its
/// leading `imap.` swapped for `smtp.` — the near-universal convention.
pub fn creds() -> Result<MailCreds, String> {
    let raw = secrets::get_integration_token("email")
        .ok_or("Email isn't connected — add your address, app password and IMAP host in Integrations.")?;
    let v: Value = serde_json::from_str(&raw).map_err(|_| "The stored email credential is malformed — reconnect it.".to_string())?;
    let get = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let imap_host = get("imap_host");
    let address = get("address");
    let password = get("app_password");
    if address.is_empty() || password.is_empty() || imap_host.is_empty() {
        return Err("Email needs an address, an app password and an IMAP host.".into());
    }
    let smtp_host = {
        let s = get("smtp_host");
        if s.is_empty() {
            imap_host.strip_prefix("imap.").map(|rest| format!("smtp.{rest}")).unwrap_or_else(|| imap_host.clone())
        } else {
            s
        }
    };
    Ok(MailCreds { address, password, imap_host, smtp_host })
}

/// Open an authenticated IMAP session (implicit TLS on 993).
pub(super) fn session(c: &MailCreds) -> Result<imap::Session<imap::Connection>, String> {
    // IMAP opens its own socket, so it can't be intercepted the way the HTTP
    // client is — it has to declare itself instead. See `core::egress::allow`.
    crate::core::egress::allow("email", &c.imap_host, "imap", 993)?;
    let client = imap::ClientBuilder::new(c.imap_host.as_str(), 993)
        .connect()
        .map_err(|e| format!("Couldn't reach {}:993 — check the IMAP host. ({e})", c.imap_host))?;
    client
        .login(&c.address, &c.password)
        .map_err(|(e, _)| format!("The mail server rejected the login — use an app password, not your account password. ({e})"))
}

/// Confirm the credentials work; returns the address plus the mailbox count.
pub fn verify_creds(c: &MailCreds) -> Result<String, String> {
    let mut s = session(c)?;
    let boxes = s.list(None, Some("*")).map_err(|e| format!("Connected, but couldn't list mailboxes: {e}"))?;
    let n = boxes.len();
    let _ = s.logout();
    Ok(format!("{} · {n} mailboxes", c.address))
}

/// Decode a header value's charset-encoded words well enough for a title, and
/// collapse the folding whitespace IMAP headers use.
pub(super) fn header_value(raw: &str, name: &str) -> String {
    let lower_name = format!("{}:", name.to_lowercase());
    for line in raw.split("\r\n") {
        if line.to_lowercase().starts_with(&lower_name) {
            return line[lower_name.len()..].trim().to_string();
        }
    }
    String::new()
}

/// Pull the most recent inbox messages into the Memory Tree.
pub fn sync(memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let c = creds()?;
    let mut s = session(&c)?;
    let mailbox = s.select("INBOX").map_err(|e| format!("Couldn't open INBOX: {e}"))?;
    let total = mailbox.exists;
    if total == 0 {
        let _ = s.logout();
        return Ok(SyncOutcome::default());
    }
    // The newest 25 messages by sequence number.
    let start = total.saturating_sub(24).max(1);
    let range = format!("{start}:{total}");
    let messages = s
        .fetch(&range, "(UID ENVELOPE BODY.PEEK[HEADER.FIELDS (SUBJECT FROM DATE)])")
        .map_err(|e| format!("Couldn't fetch messages: {e}"))?;

    let mut out = SyncOutcome::default();
    for m in messages.iter() {
        let uid = m.uid.unwrap_or(0);
        if uid == 0 {
            continue;
        }
        let raw = m.header().map(|h| String::from_utf8_lossy(h).to_string()).unwrap_or_default();
        let subject = {
            let s = header_value(&raw, "Subject");
            if s.is_empty() { "(no subject)".to_string() } else { s }
        };
        let from = header_value(&raw, "From");
        let date = header_value(&raw, "Date");

        let filename = format!("email-{uid}.md");
        let existed = memory.vault_path.join(&filename).exists();
        let input = MemoryInput {
            filename: Some(filename),
            title: format!("Email: {}", subject.chars().take(120).collect::<String>()),
            category: "Email".into(),
            tags: vec!["email".into(), "imap".into()],
            summary: if from.is_empty() { date.clone() } else { format!("from {from}") },
            body: format!("**{subject}**\n\n- From: {from}\n- Date: {date}"),
            origin: Origin::External,
            key: None,
            supersedes: None,
        };
        if memory.save(&input).is_ok() && !existed {
            out.created += 1;
            out.titles.push(format!("Email: {subject}"));
        }
    }
    let _ = s.logout();
    Ok(out)
}

/// The message `send_reply` sends, built apart from the sending so its headers can be
/// checked without a mail server.
pub(super) fn build(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
    references: Option<&str>,
) -> Result<lettre::Message, String> {
    let mut b = lettre::Message::builder()
        .from(from.parse().map_err(|e| format!("Bad sender address: {e}"))?)
        .to(to.parse().map_err(|e| format!("Bad recipient '{to}': {e}"))?)
        .subject(subject);
    // A reply names the message it answers, so it lands in the sender's thread rather than
    // arriving as a new conversation with a "Re:" in front of it.
    if let Some(id) = in_reply_to.map(str::trim).filter(|s| !s.is_empty()) {
        b = b.in_reply_to(id.to_string());
    }
    if let Some(r) = references.map(str::trim).filter(|s| !s.is_empty()) {
        b = b.references(r.to_string());
    }
    b.body(body.to_string()).map_err(|e| format!("Couldn't build the message: {e}"))
}

/// Send a message over SMTP (submission port 587 + STARTTLS), threaded under `in_reply_to`
/// when it answers one.
pub fn send_reply(
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
    references: Option<&str>,
) -> Result<String, String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{SmtpTransport, Transport};

    let c = creds()?;
    let email = build(&c.address, to, subject, body, in_reply_to, references)?;

    crate::core::egress::allow("email", &c.smtp_host, "smtp", 587)?;
    let mailer = SmtpTransport::starttls_relay(&c.smtp_host)
        .map_err(|e| format!("Couldn't reach {}: {e}", c.smtp_host))?
        .credentials(Credentials::new(c.address.clone(), c.password.clone()))
        .build();

    mailer.send(&email).map_err(|e| format!("The mail server refused the message: {e}"))?;
    Ok(format!("Emailed {to}."))
}
