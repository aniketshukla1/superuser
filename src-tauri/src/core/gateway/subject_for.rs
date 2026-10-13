use super::*;

/// The facts one action is judged on, written the way the trail records them.
pub fn subject_for(name: &str, args: &Value, mcp: Option<&McpCall>, tainted: bool) -> Subject {
    let s = |k: &str| args[k].as_str().unwrap_or("").trim().to_string();
    let intent = intent_of(name, args, mcp).to_string();
    let risk = match mcp {
        Some(m) => if m.effect == "read" { "read" } else { "external" }.to_string(),
        None => catalogue_risk().get(name).map(|r| risk_name(*r)).unwrap_or("").to_string(),
    };
    let mut out = Subject { tool: name.to_string(), intent, risk, tainted, ..Default::default() };
    if let Some(m) = mcp {
        out.mcp_server = m.server.clone();
        out.mcp_tool = m.tool.clone();
        out.mcp_effect = m.effect.clone();
        out.target = m.server.clone();
        return out;
    }
    match name {
        "run_shell" | "process_start" => {
            out.command = s("command");
            out.target = "shell".into();
        }
        // Typed into a program that is already running — which executes it.
        "process_send" => {
            out.command = s("input");
            out.target = s("handle");
        }
        "process_stop" | "process_read" => out.target = s("handle"),
        "request_tool" => out.target = format!("install {}", s("name")),
        "web_fetch" | "browser_open" | "browser_navigate" => {
            let url = crate::core::browser::normalize_url(&s("url"));
            out.page = audit::clean_url(&url);
            out.target = boundaries::host_of(&out.page);
        }
        // Typed text is never written down — its length is. Which element it went into is
        // filled in by `with_browser_element`, from the browser's own snapshot.
        "browser_type" => {
            out.element_ref = s("ref");
            out.chars = Some(args["text"].as_str().unwrap_or("").chars().count() as u64);
            if args["submit"].as_bool().unwrap_or(false) {
                out.key = "Enter".into();
            }
        }
        "browser_click" => out.element_ref = s("ref"),
        "browser_key" => {
            out.element_ref = s("ref");
            out.key = crate::core::agentbrowser::keys::press(&s("key")).map(|p| p.key).unwrap_or_else(|| s("key"));
        }
        "web_search" => {
            // The query is never written down: it left the machine, and the Network
            // page's rule is that what left is recorded by host, never by content.
            out.target = "web search".into();
            out.chars = Some(s("query").chars().count() as u64);
        }
        "send_message" => {
            out.target = s("target");
            out.chars = Some(args["text"].as_str().unwrap_or("").chars().count() as u64);
        }
        "schedule_task" => {
            let when = [s("cron"), s("at")].into_iter().find(|w| !w.is_empty()).unwrap_or_default();
            out.target = format!("{} ({when})", s("title"));
            out.chars = Some(s("instructions").chars().count() as u64);
        }
        "watch_for" => out.target = s("question").chars().take(120).collect(),
        "knowledge_write" => out.target = s("title"),
        "connector_sync" | "connector_search" | "connector_changes" => out.target = s("connector"),
        "spawn_subagent" => {
            let tasks = args["tasks"].as_array().map(|t| t.len()).unwrap_or(1);
            out.target = format!("{tasks} task(s)");
        }
        // What is being added, by name — never its instructions, which are the draft's
        // content and belong in front of the person rather than in the trail.
        "save_skill" => out.target = args["name"].as_str().unwrap_or_default().trim().chars().take(64).collect(),
        "save_persona" => out.target = args["id"].as_str().unwrap_or_default().trim().chars().take(64).collect(),
        // Who was asked, and how much was said to them — never the words, which are this
        // conversation's content however they were summarised.
        "ask_coworker" => {
            out.target = args["coworker"].as_str().unwrap_or_default().trim().chars().take(64).collect();
            out.chars = Some(args["task"].as_str().unwrap_or_default().chars().count() as u64);
        }
        "computer_use" => {
            out.target = "desktop".into();
            out.chars = Some(s("task").chars().count() as u64);
        }
        "write_file" => {
            out.file = s("path");
            out.target = out.file.clone();
            out.chars = Some(args["content"].as_str().unwrap_or("").chars().count() as u64);
        }
        _ => {
            if !s("path").is_empty() {
                out.file = s("path");
                out.target = out.file.clone();
            } else if matches!(name, "git_status" | "git_diff" | "git_log") {
                out.target = "git".into();
            }
        }
    }
    out
}

/// Fill in what a browser action is aimed at from the browser itself: the element its ref
/// names in that assistant's latest snapshot, and the page it is on.
///
/// This is the half of the gateway that makes element rules mean anything. The
/// model's arguments carry only a ref; the label a rule reads comes from the snapshot this
/// process took, so an assistant cannot get past "never click Submit" by describing the
/// button as something else. A ref that names nothing leaves the element empty — the
/// action itself is then refused for naming nothing, after the decision is recorded.
pub fn with_browser_element(mut subject: Subject, persona: &str) -> Subject {
    if !crate::core::agent::is_browser_tool(&subject.tool) {
        return subject;
    }
    if !subject.element_ref.is_empty() {
        if let Some((el, url)) = crate::core::agentbrowser::element_for(persona, &subject.element_ref) {
            subject.element_role = el.role;
            subject.element_name = el.name;
            subject.element_type = el.input_type;
            if subject.page.is_empty() {
                subject.page = audit::clean_url(&url);
            }
        }
    }
    if subject.page.is_empty() {
        if let Some(url) = crate::core::agentbrowser::current_url(persona) {
            subject.page = audit::clean_url(&url);
        }
    }
    if subject.target.is_empty() {
        subject.target = boundaries::host_of(&subject.page);
    }
    subject
}

/// Whether an *allowed* action is written down.
///
/// Every refusal is, whatever it was. Allowed actions are recorded when they reach
/// past the conversation — a file, a command, the network, another system, another
/// agent, the future. Looking something up in this app's own memory, drawing a card
/// and ticking a board item are not recorded when they are allowed: a trail that
/// logs every recall is a trail nobody reads.
pub fn recorded(intent: &str) -> bool {
    !matches!(intent, "read" | "show" | "track")
}

/// Permission to go ahead with one action, carrying what [`after`] needs.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub subject: Subject,
    pub decision: Decision,
    /// Whether a decision row was written — and so whether an outcome row may follow.
    pub recorded: bool,
}

/// Decide, and write the decision down. `Err` is the sentence to hand the assistant
/// instead of running the tool.
pub fn before(data_dir: &Path, facts: &Facts, subject: Subject) -> Result<Ticket, String> {
    let loaded = boundaries::load(data_dir);
    let ctx = boundaries::context(&subject, &facts.persona, &facts.session, &facts.initiator);
    let decision = loaded.decide(&ctx);
    let write = !decision.allowed || recorded(&subject.intent);
    if write {
        let mut e = Entry::event(if decision.carried_out { "action.allowed" } else { "action.refused" });
        e.actor = facts.persona.clone();
        e.persona = facts.persona.clone();
        e.session = facts.session.clone();
        e.initiator = Some(facts.initiator.clone());
        e.target = subject.target.clone();
        e.ok = decision.carried_out;
        e.detail = match (decision.allowed, decision.carried_out) {
            (true, _) => boundaries::what(&subject),
            (false, true) => format!("{} — a boundary would refuse this; recording only, so it went ahead", boundaries::what(&subject)),
            (false, false) => format!("{} — refused", boundaries::what(&subject)),
        };
        e.subject = Some(subject.clone());
        e.decision = Some(decision.clone());
        audit::append(data_dir, e);
    }
    if !decision.carried_out {
        crate::trace!("gateway", "refused {} ({}) by {:?}", subject.tool, subject.intent, decision.rule);
        return Err(boundaries::refusal(&subject, &decision, loaded.problem.as_deref()));
    }
    Ok(Ticket { subject, decision, recorded: write })
}
