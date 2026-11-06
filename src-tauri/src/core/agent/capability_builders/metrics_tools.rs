use super::super::*;

pub(in crate::core::agent) fn git_tools() -> Vec<ToolSpec> {
    vec![
        tool("git_status", "Show the working tree status.", "git", RiskClass::Read, params(json!({}), &[])),
        tool("git_diff", "Show unstaged changes, optionally for one path.", "git", RiskClass::Read,
            params(json!({"path": {"type": "string", "description": "Limit the diff to one file or directory. Omit for every unstaged change."}}), &[])),
        tool(
        "ask_form",
        "Ask the user for VALUES only they hold, and wait — what they type comes back as this tool's result. \
         For a target margin, a start date, an account id, the address of a staging box: facts with nothing to enumerate, \
         where offering \"2 to 4 likely answers\" would mean inventing them. \
         \
         NOT for judgment. If you can list the plausible answers, you owe the user options — use `ask_user`, where writing \
         those options is the work. A form of empty fields where a decision was needed moves your thinking onto them. \
         \
         NOT for anything findable. A file path, a filename, a sheet name, a column name: go and look with `list_dir`, \
         `grep`, `sheet_profile` or `read_file`. A form is a tempting place to ask for a pile of those at once, and it is \
         the same failure as asking for one. \
         \
         At most six fields, and ask only for what you cannot proceed without. Each: {\"name\": str (key the answer comes \
         back under), \"label\": str, \"kind\": text|number|date, \"placeholder\": str, \"help\": str, \"required\": bool}.",
        "ask",
        RiskClass::Read,
        params(
            json!({
                "title": {"type": "string", "description": "What the form is for, in a few words."},
                "intro": {"type": "string", "description": "One line saying why you need these."},
                "fields": {"type": "array", "description": "One to six fields.", "items": {"type": "object"}}
            }),
            &["fields"],
        ),
    ),
    tool("git_log", "Show recent commit history.", "git", RiskClass::Read,
            params(json!({"max": {"type": "integer", "description": "How many commits to return, newest first. Defaults to a short list."}}), &[])),
    ]
}

/// The Chief of Staff's one tool (`core::chief_of_staff`). Read-only: what counts as recent is
/// decided here, and the items are numbered so every claim can cite one.
pub(in crate::core::agent) fn activity_tools() -> Vec<ToolSpec> {
    vec![tool(
        "activity_since",
        "Everything that arrived or was written in a recent window — mail, calendars, the apps the person connected, their own \
         notes — numbered A1, A2…, followed by their priorities numbered PR1, PR2…. Cite both when you map an item to a priority.",
        "activity",
        RiskClass::Read,
        params(
            json!({"since": {"type": "string", "enum": ["yesterday", "last_24h", "this_week", "last_7_days"], "description": "Where the window starts. Defaults to yesterday (its start)."}}),
            &[],
        ),
    )]
}

/// The Product Performance assistant's tools (`core::perf`), for Prometheus or anything that
/// speaks its API. Read-only, all of them: the only endpoints called are the query API's, and
/// the numbers — averages, spreads, what moved — come from code.
pub(in crate::core::agent) fn metrics_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "metrics_find",
            "Find metrics in the connected Prometheus by words in their names (\"http duration\", \"checkout\", \"gc\"), \
             with each one's help text. Give one exact metric name to see its labels and some of their values — what you can \
             group and filter by.",
            "metrics",
            RiskClass::Read,
            params(json!({"text": {"type": "string", "description": "Words to look for, or one exact metric name."}}), &["text"]),
        ),
        tool(
            "metrics_query",
            "Run a PromQL query for one moment — now unless `at` says otherwise — and get every series it returns, highest first, \
             with a link to the same query in Prometheus.",
            "metrics",
            RiskClass::Read,
            params(
                json!({
                    "query": {"type": "string", "description": "PromQL."},
                    "at": {"type": "string", "description": "When: 2026-09-23 14:00 (this computer's time zone), RFC 3339, or a duration ago like 3h. Default now."}
                }),
                &["query"],
            ),
        ),
        tool(
            "metrics_range",
            "Run a PromQL query over a time range and get each series summarised — average, min, max, last — with a link, and \
             the series kept as a CSV you can chart.",
            "metrics",
            RiskClass::Read,
            params(
                json!({
                    "query": {"type": "string", "description": "PromQL."},
                    "since": {"type": "string", "description": "Start: a duration ago like 6h or 2d, or a time. Default 1h."},
                    "until": {"type": "string", "description": "End. Default now."},
                    "step": {"type": "string", "description": "Resolution, like 1m. Leave it out to get about 240 points."}
                }),
                &["query"],
            ),
        ),
        tool(
            "metrics_compare",
            "Compare a PromQL query before and after a moment — a release, a config change, when a complaint started — for every \
             series it returns. Says of each whether it moved clearly (a sustained step well beyond its usual spread), possibly, \
             or within its usual variation, ranked, with the numbers; also series that appeared or vanished at the change. This \
             is the evidence for a hotspot: group the query by the label a hotspot could hide in (`sum by (handler)`, \
             `by (pod)`, `by (version)`). Keeps the series as a CSV you can chart.",
            "metrics",
            RiskClass::Read,
            params(
                json!({
                    "query": {"type": "string", "description": "PromQL, usually grouped by a label."},
                    "change_at": {"type": "string", "description": "When the change happened: 2026-09-23 14:00 (this computer's time zone), RFC 3339, or a duration ago like 20h."},
                    "window": {"type": "string", "description": "How long to look on each side, like 1h or 6h. Default 1h."}
                }),
                &["query", "change_at"],
            ),
        ),
        tool(
            "metrics_alerts",
            "Alerts firing or pending in Prometheus now: name, severity, since when, what they say, and where. Read-only — nothing \
             here silences or changes an alert.",
            "metrics",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "perf_report",
            "The health report: every signal the person set in Settings ▸ General ▸ Product Performance, over the last `window` \
             against the one before, grouped into clear shifts, possible shifts and steady, with numbers and links.",
            "metrics",
            RiskClass::Read,
            params(json!({"window": {"type": "string", "description": "Each side's length, like 7d or 24h. Default 7d."}}), &[]),
        ),
    ]
}

/// The Expense Manager's one tool (`core::expenses`). Read-only: the ledger is written by the
/// intake, never by a model, and the totals come from code so they reconcile.
pub(in crate::core::agent) fn expense_tools() -> Vec<ToolSpec> {
    vec![tool(
        "expense_summary",
        "Summarize the expense ledger for a period: totals by category and currency (each listing the receipts it adds up), \
         every receipt, what is missing a category or currency, possible duplicates, and the expense policy with its lines \
         numbered P1, P2… for citing. Give its totals as they are; they reconcile to the receipts.",
        "expenses",
        RiskClass::Read,
        params(
            json!({
                "period": {"type": "string", "enum": ["this_week", "last_week", "this_month", "last_month"], "description": "Which span of days. Defaults to this_week (Monday to today)."},
                "from": {"type": "string", "description": "Instead of a period: the first day, YYYY-MM-DD."},
                "to": {"type": "string", "description": "Instead of a period: the last day, YYYY-MM-DD."}
            }),
            &[],
        ),
    )]
}

pub(in crate::core::agent) fn todo_tools() -> Vec<ToolSpec> {
    // Param key is `todos`, NOT `items` (a top-level `items` key shadows a map method
    // in some hosted chat templates and 400s the request).
    vec![tool("todo_write", "Set the task list. Send every todo on each call, because this replaces the whole list; the Progress panel is drawn from it.", "todo", RiskClass::Read,
        params(json!({"todos":{
            "type": "array",
            "description": "The WHOLE list, every call — it replaces the previous one rather than adding to it. Carry finished items forward with status \"done\" or they disappear from the panel.",
            "items":{"type":"object","properties":{
                "content":{"type":"string","description":"What the step is, in a few words."},
                "status":{"type":"string","enum":["pending","in_progress","done"],"description":"Keep at most one item \"in_progress\"."}
            },"required":["content","status"]}}}), &["todos"]))]
}

pub(in crate::core::agent) fn plan_tools() -> Vec<ToolSpec> {
    vec![tool("propose_plan", "Propose a step-by-step plan for the user to approve before you act. Markdown.", "plan", RiskClass::Read,
        params(json!({"plan": {"type": "string", "description": "The plan as markdown, one step per line. The user approves or rejects it before anything runs."}}), &["plan"]))]
}
