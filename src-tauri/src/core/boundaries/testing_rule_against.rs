//! testing a rule against history

use super::*;

/// One recorded action a candidate set of boundaries would decide differently.
#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub at: f64,
    pub session: String,
    pub persona: String,
    pub tool: String,
    pub intent: String,
    /// The action in words — see [`what`].
    pub what: String,
    pub was: &'static str,
    pub would: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rule: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rule_name: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Report {
    /// Recorded actions replayed.
    pub scanned: usize,
    pub would_refuse: usize,
    pub would_allow: usize,
    pub unchanged: usize,
    /// Every change, up to [`CHANGES_CAP`]; the counts cover everything scanned.
    pub changes: Vec<Change>,
}

pub const CHANGES_CAP: usize = 50;

/// How these boundaries would have ruled on actions already taken.
///
/// This replays rather than simulates: each recorded row is turned back into a context
/// by [`context`], the same function the gateway used at the time. Only decision rows are
/// replayed — a failure or a stop is a second row about an action already counted.
pub fn replay(candidate: &Boundaries, rows: &[Entry]) -> Report {
    let loaded = compile(candidate.clone(), None);
    let mut report = Report::default();
    for row in rows {
        let (Some(subject), Some(decision)) = (&row.subject, &row.decision) else { continue };
        if !matches!(row.event_name(), "action.allowed" | "action.refused") {
            continue;
        }
        report.scanned += 1;
        let initiator = row.initiator.clone().unwrap_or_else(|| Initiator::from_session(&row.session));
        let ctx = context(subject, &row.persona, &row.session, &initiator);
        let would = loaded.decide(&ctx);
        // "Was" is what the rules decided at the time, not whether it ran: a record-
        // mode refusal ran, and a candidate that refuses it too has changed nothing.
        let was_refused = !decision.allowed;
        let would_refuse = !would.allowed;
        if was_refused == would_refuse {
            report.unchanged += 1;
            continue;
        }
        if would_refuse {
            report.would_refuse += 1;
        } else {
            report.would_allow += 1;
        }
        if report.changes.len() < CHANGES_CAP {
            report.changes.push(Change {
                at: row.at,
                session: row.session.clone(),
                persona: row.persona.clone(),
                tool: subject.tool.clone(),
                intent: subject.intent.clone(),
                what: what(subject),
                was: if was_refused { "refused" } else { "allowed" },
                would: if would_refuse { "refused" } else { "allowed" },
                rule: would.rule,
                rule_name: would.rule_name,
            });
        }
    }
    report
}

/// An action in a few words, for a refusal and for the Test list.
pub fn what(s: &Subject) -> String {
    let short = |t: &str, n: usize| {
        let t = t.trim();
        if t.chars().count() > n { format!("{}…", t.chars().take(n).collect::<String>()) } else { t.to_string() }
    };
    match s.intent.as_str() {
        "run_command" if !s.command.is_empty() => format!("running `{}`", short(&s.command, 120)),
        "write_file" if !s.file.is_empty() => format!("writing {}", s.file),
        "read_file" if !s.file.is_empty() => format!("reading {}", s.file),
        "list_files" => format!("listing {}", if s.file.is_empty() { "the working folder" } else { &s.file }),
        "navigate" if !s.page.is_empty() => format!("opening {}", s.page),
        "search" => "searching the web".into(),
        "send" => format!("sending a message to {}", if s.target.is_empty() { "someone" } else { &s.target }),
        "schedule" => format!("setting up “{}”", short(&s.target, 80)),
        "remember" => format!("saving “{}” to the knowledge base", short(&s.target, 80)),
        "delegate" => match s.tool.as_str() {
            "ask_coworker" => format!("asking {} to do something", if s.target.is_empty() { "another assistant" } else { &s.target }),
            _ => "handing work to a subagent".into(),
        },
        "author" => match s.tool.as_str() {
            "save_persona" => format!("adding the assistant “{}”", short(&s.target, 60)),
            _ => format!("adding the skill “{}”", short(&s.target, 60)),
        },
        "activate" | "type" => {
            let on = if s.target.is_empty() { String::new() } else { format!(" on {}", s.target) };
            let el = if s.element_name.is_empty() { format!("a {}", if s.element_role.is_empty() { "control" } else { &s.element_role }) } else { format!("“{}”", s.element_name) };
            match (s.intent.as_str(), s.tool.as_str()) {
                ("activate", "browser_click") => format!("clicking {el}{on}"),
                ("activate", "browser_type") => format!("typing into {el}{on} and pressing Enter"),
                ("type", "browser_type") => format!("typing into {el}{on}"),
                _ if !s.key.is_empty() && !s.element_name.is_empty() => format!("pressing {} on {el}{on}", s.key),
                _ if !s.key.is_empty() => format!("pressing {}{on}", s.key),
                _ => format!("using {el}{on}"),
            }
        }
        "use_computer" => "driving the desktop".into(),
        "read_tool" | "write_tool" if !s.mcp_tool.is_empty() => format!("{} in {}", s.mcp_tool, s.mcp_server),
        "read_tool" if !s.target.is_empty() => format!("syncing {}", s.target),
        _ => format!("calling {}", s.tool),
    }
}

/// The result an assistant receives when a boundary stops it.
///
/// Starts with `Refused.` so the transcript, the plan runner and the scoreboard can
/// tell a boundary that held from a tool that broke. The instruction after it is the
/// one assistants most need: say so, and stop — do not look for
/// another way to do the same thing.
pub fn refusal(s: &Subject, d: &Decision, problem: Option<&str>) -> String {
    let action = what(s);
    let by = if !d.rule_name.is_empty() {
        format!("the boundary “{}” (`{}`)", d.rule_name, d.rule)
    } else if !d.rule.is_empty() {
        format!("the boundary `{}`", d.rule)
    } else {
        String::new()
    };
    let why = if let Some(p) = problem {
        format!("the user's boundaries file can't be read ({p}), and while it can't be read every action is refused")
    } else if d.source == "default" {
        "nothing on the user's allow list permits it".to_string()
    } else if !d.reason.is_empty() {
        format!("{by} can't be read ({}), and a boundary that can't be read refuses what it might cover", d.reason)
    } else {
        format!("{by} forbids it")
    };
    format!(
        "Refused. {} was stopped before it happened: {why}. This is a limit the user set, not an error — \
         don't try another way to do the same thing, and don't retry it. Tell the user what you were about \
         to do and that their boundaries stopped it. If it should be allowed, they can change that in \
         Settings ▸ Boundaries.",
        capitalise(&action)
    )
}

pub(super) fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
