use super::*;

/// Write the outcome down, when it was anything but done.
pub fn after(data_dir: &Path, facts: &Facts, ticket: &Ticket, result: &str, stopped: bool) {
    if !ticket.recorded {
        return;
    }
    let (event, why) = if stopped {
        ("action.stopped", "the user pressed Stop while it was running".to_string())
    } else {
        match outcome_of(result) {
            Some(o) => o,
            None => return,
        }
    };
    let mut e = Entry::event(event);
    e.actor = facts.persona.clone();
    e.persona = facts.persona.clone();
    e.session = facts.session.clone();
    e.initiator = Some(facts.initiator.clone());
    e.target = ticket.subject.target.clone();
    e.ok = false;
    e.detail = format!("{} — {why}", boundaries::what(&ticket.subject));
    e.subject = Some(ticket.subject.clone());
    audit::append(data_dir, e);
}

/// What a tool's result says happened, when it was not simply done.
///
/// Tools answer in text, and these prefixes are the ones the approval engine and
/// the tools themselves have always used; `run_plan_steps` already stops on the same
/// ones. Output is never copied into the trail — only the first line of a failure,
/// which is the tool's own words about why.
pub fn outcome_of(result: &str) -> Option<(&'static str, String)> {
    let head = result.trim_start();
    let first: String = head.lines().next().unwrap_or("").chars().take(200).collect();
    let low = head.to_lowercase();
    if low.starts_with("parked for approval") {
        return Some(("action.parked", "waiting for the user's approval in the Inbox".into()));
    }
    if low.starts_with("denied by the user") {
        return Some(("action.declined", "the user said no".into()));
    }
    if low.starts_with("not run") || low.starts_with("not sent") || low.starts_with("not shown") {
        return Some(("action.declined", first));
    }
    if status_of(result) == "error" {
        return Some(("action.failed", first));
    }
    None
}

/// How a result reads: `ok`, `error`, or `refused` — a boundary that held, which is
/// not the same thing as a tool that broke, and must not be drawn as one.
pub fn status_of(out: &str) -> &'static str {
    let head = out.trim_start().chars().take(120).collect::<String>().to_lowercase();
    if head.starts_with("refused.") {
        "refused"
    } else if head.starts_with("error") || head.starts_with("unknown ") || head.contains("failed:") {
        "error"
    } else {
        "ok"
    }
}
