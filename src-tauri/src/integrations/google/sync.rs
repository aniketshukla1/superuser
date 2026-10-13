use super::*;

/// Sync recent Gmail messages (last 7 days, metadata + snippet only).
pub async fn sync_gmail(memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let token = access_token().await?;
    let client = crate::core::egress::client("gmail");
    let list = gapi(&client, &token, "https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=15&q=newer_than:7d").await?;
    let mut out = SyncOutcome::default();

    if let Some(msgs) = list["messages"].as_array() {
        for m in msgs {
            let id = m["id"].as_str().unwrap_or("");
            if id.is_empty() { continue; }
            let filename = format!("gmail-{id}.md");
            if memory.vault_path.join(&filename).exists() { continue; } // skip already-synced (saves API calls)

            let msg = gapi(&client, &token, &format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date")).await?;
            let mut subject = String::new();
            let mut from = String::new();
            let mut date = String::new();
            if let Some(headers) = msg["payload"]["headers"].as_array() {
                for h in headers {
                    match h["name"].as_str() {
                        Some("Subject") => subject = h["value"].as_str().unwrap_or("").into(),
                        Some("From") => from = h["value"].as_str().unwrap_or("").into(),
                        Some("Date") => date = h["value"].as_str().unwrap_or("").into(),
                        _ => {}
                    }
                }
            }
            let snippet = msg["snippet"].as_str().unwrap_or("");
            if subject.is_empty() { subject = "(no subject)".into(); }

            let input = MemoryInput {
                filename: Some(filename),
                title: format!("Email: {subject}"),
                category: "Gmail".into(),
                tags: vec!["gmail".into(), "email".into()],
                summary: format!("From {from} · {date}"),
                body: format!("**{subject}**\n\n- From: {from}\n- Date: {date}\n\n{snippet}"),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() {
                out.created += 1;
                out.titles.push(format!("New email: {subject}"));
            }
        }
    }
    Ok(out)
}

/// Sync calendar events (past 7 days through the next 14).
pub async fn sync_calendar(memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let token = access_token().await?;
    let client = crate::core::egress::client("calendar");
    let now = chrono::Utc::now();
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=25",
        urlencode(&(now - chrono::Duration::days(7)).to_rfc3339()),
        urlencode(&(now + chrono::Duration::days(14)).to_rfc3339())
    );
    let list = gapi(&client, &token, &url).await?;
    let mut out = SyncOutcome::default();

    if let Some(items) = list["items"].as_array() {
        for ev in items {
            let id = ev["id"].as_str().unwrap_or("");
            if id.is_empty() { continue; }
            let title = ev["summary"].as_str().unwrap_or("(no title)");
            let start = ev["start"]["dateTime"].as_str().or(ev["start"]["date"].as_str()).unwrap_or("");
            let loc = ev["location"].as_str().unwrap_or("");
            let attendees = ev["attendees"].as_array().map(|a| a.iter().filter_map(|x| x["email"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();

            let filename = format!("calendar-{id}.md");
            let existed = memory.vault_path.join(&filename).exists();
            let input = MemoryInput {
                filename: Some(filename),
                title: format!("Event: {title}"),
                category: "Calendar".into(),
                tags: vec!["calendar".into(), "event".into()],
                summary: format!("{start}{}", if loc.is_empty() { String::new() } else { format!(" · {loc}") }),
                body: format!("**{title}**\n\n- When: {start}\n- Where: {}\n- Attendees: {}\n\n{}", if loc.is_empty() { "—" } else { loc }, if attendees.is_empty() { "—".into() } else { attendees }, ev["description"].as_str().unwrap_or("")),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() && !existed {
                out.created += 1;
                out.titles.push(format!("Upcoming: {title}"));
            }
        }
    }
    Ok(out)
}
