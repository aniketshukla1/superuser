use super::super::*;

pub(in crate::core::agent) fn knowledge_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "knowledge_write",
            "Write or update a topic page in the user's knowledge base — a lasting reference on one subject \
             (a concept, a person, a project, a system). Writing the same title again REPLACES that page, so \
             include everything it should say, not just the new part. Link related pages as [[Their Title]]. \
             Use this for knowledge worth looking up again; not for what happened today.",
            "knowledge",
            RiskClass::WriteLocal,
            params(
                json!({
                    "title": {"type": "string", "description": "The topic. Reusing a title edits that page."},
                    "category": {"type": "string", "description": "Grouping, e.g. concepts, people, projects, tools."},
                    "body": {"type": "string", "description": "The full markdown content of the page."}
                }),
                &["title", "body"],
            ),
        ),
        tool(
            "knowledge_search",
            "Search the user's knowledge base for topic pages. Check here before answering a question about \
             something you may have written up before.",
            "knowledge",
            RiskClass::Read,
            params(json!({"query": {"type": "string"}}), &["query"]),
        ),
        tool(
            "knowledge_read",
            "Read one topic page in full, by its title.",
            "knowledge",
            RiskClass::Read,
            params(json!({"title": {"type": "string"}}), &["title"]),
        ),
    ]
}

pub(in crate::core::agent) fn schedule_tools() -> Vec<ToolSpec> {
    vec![
        tool(
        "watch_for",
        "Keep an eye on something and speak up only when it changes — 'tell me if anything urgent \
         lands in my inbox', 'flag new issues on the deploy repo'. Use this instead of \
         `schedule_task` when the user cares about *a change*, not about a time: a watch costs \
         nothing on the many checks where nothing has happened, while a scheduled task pays for \
         a full run every time it fires. \
         `question` is the standing question, in the user's own words. `sources` names the \
         connectors whose changes feed it (omit for the whole vault). Set `writes` when acting on \
         it could mean sending, posting or filing something — the user is then always asked first.",
        "schedule",
        RiskClass::WriteLocal,
        params(
            json!({
                "question": {"type": "string", "description": "What to watch for, in the user's words."},
                "sources": {"type": "array", "items": {"type": "string"}, "description": "Connector ids, e.g. [\"gmail\"]. Omit for everything."},
                "writes": {"type": "boolean", "description": "True if acting on it could affect anything outside this machine."}
            }),
            &["question"],
        ),
    ),
    tool(
        "schedule_task",
        "Schedule work to happen later, or on a repeating schedule — reminders, daily briefs, periodic checks. \
         Give either `cron` (e.g. `0 9 * * 1-5` for 9am on weekdays) or `at` (an ISO time like `2026-08-14T15:00:00`). \
         `instructions` is what you'll be asked to do when it fires, so write it to stand alone: the run cannot see this conversation. \
         Its answer comes back into this conversation each time it runs.",
        "schedule",
        RiskClass::WriteLocal,
        params(
            json!({
                "title": {"type": "string", "description": "Short label, e.g. 'Morning brief'."},
                "instructions": {"type": "string", "description": "Self-contained instructions for the run."},
                "cron": {"type": "string", "description": "Cron expression for a repeating task."},
                "at": {"type": "string", "description": "ISO timestamp for a one-off."}
            }),
            &["title", "instructions"],
        ),
    ),
    ]
    .into_iter()
    // One capability, all four verbs: an assistant that can make an automation should be
    // able to answer "what have I got running?" without a second grant.
    .chain(automation_tools())
    .collect()
}

pub(in crate::core::agent) fn connector_tools() -> Vec<ToolSpec> {
    vec![
        tool("list_connectors", "List the user's connected apps (GitHub, Gmail, Calendar, Notion, Linear, Stripe, …) that you can pull context from.", "connectors", RiskClass::Read,
            params(json!({}), &[])),
        tool("connector_search", "Search a connected app's synced content (issues, emails, events, pages) for relevant items.", "connectors", RiskClass::Read,
            params(json!({"connector":{"type":"string"},"query":{"type":"string"}}), &["query"])),
        tool("connector_sync", "Pull the latest content from a connected app into the user's memory, then search it.", "connectors", RiskClass::External,
            params(json!({"connector":{"type":"string"}}), &["connector"])),
        // Read-class despite moving a marker: the only thing it writes is a note of
        // what you have already been shown, which is bookkeeping about the agent,
        // not a change to the user's data.
        tool(
            "connector_changes",
            "What's new in a connected app since you last looked — added, changed and removed items. \
             Use this instead of re-reading everything: a second call returns only what arrived since the first, \
             so a recurring task never reports the same item twice. Pass `peek: true` to look without marking it seen.",
            "connectors",
            RiskClass::Read,
            params(
                json!({
                    "connector": {"type": "string", "description": "The app id, e.g. 'gmail', 'github', 'slack'."},
                    "peek": {"type": "boolean", "description": "Look without consuming the delta. Default false."}
                }),
                &["connector"],
            ),
        ),
    ]
}

/// The assistant tools that reach one connector, exactly as the model is offered them.
///
/// Superuser's tools are not per-connector — `connector_sync` takes the app as an
/// argument — so what a connector answers to is decided by what its client can do:
/// Prometheus is queried live by the metrics tools, a synced connector is read through
/// the memory it syncs into, and a sendable one can be messaged. Only tools that act on
/// one named connector count; `list_connectors` names them all and reads none.
pub fn tools_using(connector: &str) -> Vec<ToolSpec> {
    let id = crate::integrations::normalize_id(connector);
    let mut out = Vec::new();
    if id == "prometheus" {
        out.extend(metrics_tools());
    }
    if crate::integrations::syncs(id) {
        out.extend(connector_tools().into_iter().filter(|t| t.name != "list_connectors"));
    }
    if crate::core::senders::is_sendable(id) {
        out.extend(messaging_tools());
    }
    out
}

/// The closed, platform-owned capability catalog. Every id here must appear in
/// `personas::KNOWN_TOOLS` and vice versa (enforced by a test).
pub fn catalog() -> Vec<Capability> {
    vec![
        Capability { id: "files", name: "Files", description: "Read & edit files and spreadsheets in the session's workspace.", requires: &[Req::Workspace], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: files_tools },
        Capability { id: "search", name: "Search", description: "Fast code/content search (grep).", requires: &[Req::Workspace], risk: &[RiskClass::Read], build: search_tools },
        Capability { id: "shell", name: "Shell", description: "Run shell commands in a persistent session.", requires: &[Req::Executor], risk: &[RiskClass::Exec], build: shell_tools },
        Capability { id: "git", name: "Git", description: "Inspect git state and history.", requires: &[Req::Workspace], risk: &[RiskClass::Read], build: git_tools },
        Capability { id: "todo", name: "Task list", description: "Maintain a visible task/progress list.", requires: &[Req::Todo], risk: &[RiskClass::Read], build: todo_tools },
        Capability { id: "plan", name: "Plan", description: "Propose a plan for approval before acting.", requires: &[], risk: &[RiskClass::Read], build: plan_tools },
        Capability { id: "ask", name: "Ask the user", description: "Ask the user a question mid-task.", requires: &[], risk: &[RiskClass::Read], build: ask_tools },
        Capability { id: "web", name: "Web", description: "Search and read the live web.", requires: &[], risk: &[RiskClass::External], build: web_tools },
        Capability { id: "recall", name: "Recall output", description: "Read back tool output that was truncated out of the transcript.", requires: &[], risk: &[RiskClass::Read], build: recall_output_tools },
        Capability { id: "recall_memory", name: "Memory", description: "Recall relevant context from the big-brain.", requires: &[], risk: &[RiskClass::Read], build: recall_memory_tools },
        Capability { id: "computer", name: "Computer use", description: "Drive the desktop to accomplish a task.", requires: &[], risk: &[RiskClass::Exec], build: computer_tools },
        Capability { id: "messaging", name: "Messaging", description: "Send messages on the user's behalf (approval-gated).", requires: &[], risk: &[RiskClass::External], build: messaging_tools },
        Capability { id: "subagent", name: "Subagents", description: "Delegate focused subtasks to subagents.", requires: &[], risk: &[RiskClass::Read], build: subagent_tools },
        Capability { id: "authoring", name: "Writing skills and assistants", description: "Draft a new skill or a new assistant — saved only when you press the card.", requires: &[], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: authoring_tools },
        Capability { id: "schedule", name: "Scheduling", description: "Set up reminders and recurring work, and change what is already running.", requires: &[], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: schedule_tools },
        Capability { id: "activity", name: "Recent activity", description: "Read what arrived and was written in a recent window — mail, calendars, connected apps, your notes — beside your priorities.", requires: &[], risk: &[RiskClass::Read], build: activity_tools },
        Capability { id: "metrics", name: "Metrics", description: "Query the connected Prometheus — live and read-only — and compare before and after a change.", requires: &[], risk: &[RiskClass::Read], build: metrics_tools },
        Capability { id: "talent", name: "Talent", description: "Check candidates against your ATS export and past shortlists, and keep a shortlist — contacting no one.", requires: &[], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: talent_tools },
        Capability { id: "expenses", name: "Expenses", description: "Read the expense ledger as a summary whose totals reconcile to the receipts.", requires: &[], risk: &[RiskClass::Read], build: expense_tools },
        Capability { id: "knowledge", name: "Knowledge base", description: "Keep topic pages the user can look things up in.", requires: &[], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: knowledge_tools },
        Capability { id: "connectors", name: "Connected apps", description: "Read context from the user's connected apps.", requires: &[], risk: &[RiskClass::Read, RiskClass::External], build: connector_tools },
        Capability { id: "board", name: "Work board", description: "Track work items across turns, with review before anything counts as done.", requires: &[Req::Workspace], risk: &[RiskClass::Read, RiskClass::WriteLocal], build: board_tools },
        Capability { id: "toolchain", name: "Scanner toolchain", description: "Check which security scanners are installed, and report what a missing one leaves uncovered.", requires: &[Req::Executor], risk: &[RiskClass::Read, RiskClass::Exec], build: toolchain_tools },
        Capability { id: "browser", name: "Own browser", description: "A real browser the assistant works websites in — signed in, watchable, and handed to you at a sign-in.", requires: &[], risk: &[RiskClass::Read, RiskClass::WriteLocal, RiskClass::External], build: browser_tools },
    ]
}

/// Capabilities every assistant has, without being granted them.
///
/// These are not privileges, they are manners. An assistant that cannot ask a
/// question has to guess, and one that cannot propose a plan has to either act or
/// stay silent — so leaving them to be granted per persona meant every persona in
/// the product went without: `plan` and `ask` were in the catalogue, wired, tested,
/// and reached nobody at all.
///
/// `recall` joins them on the same argument. Truncation happens to every run, and
/// the notice it leaves names a tool by name — a model told to call
/// `recall_output` that has not been granted it is worse off than one that was
/// never offered the handle, because it will spend a step finding that out. It
/// acts on nothing outside this run's own already-produced bytes.
///
/// Safe to give unconditionally because none of them act. All are `RiskClass::Read`,
/// and the real boundary is the permission MODE — `discuss` and `plan` deny
/// anything that acts at the decision layer (`core::inbox`), whatever tools are
/// present. So a discuss-only assistant gaining the ability to ask a question does
/// not gain the ability to do anything.
pub const ALWAYS_ON: &[&str] = &["plan", "ask", "recall"];
