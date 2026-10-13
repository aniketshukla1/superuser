use super::super::*;

/// Asking, as a first-class move rather than a fallback.
///
/// The description is long on purpose, and most of its length goes on **what not to
/// ask**. A model's default is to proceed on the most likely reading of an ambiguous
/// request, because that is what it is rewarded for everywhere else and the cost of
/// being wrong is invisible to it. But the opposite failure is just as bad and
/// arrives sooner: asked to be careful, a model starts asking for things it could
/// have looked up — which sheet, which file, which column — and every one of those
/// is a question that hands the user the agent's own job.
///
/// So the rule is drawn twice: ask about **judgment**, look up **fact**. And every
/// question has to arrive with the answers the agent thinks are likely, which is the
/// part that does the real work — you cannot name four candidate answers without
/// working out what the answers could be, and doing that is often enough to discover
/// you did not need to ask.
pub(in crate::core::agent) fn ask_tools() -> Vec<ToolSpec> {
    vec![refusal_tool(), tool(
        "ask_user",
        "Ask the user up to four questions and WAIT — their answers come back as this tool's result. \
         Each question must carry 2-4 options: the answers you actually believe are likely, each with a short \
         line saying what choosing it would mean. Writing the options IS the work — it forces you to work out \
         what the answers could be, and it lets the user answer in one click instead of composing a reply. \
         A question with no options is refused. The card always adds a free-text \"Other\", so never add one yourself. \
         \
         ASK ABOUT JUDGMENT, NOT FACT. Good: which of several real approaches they want, what to prioritise, \
         what a term means in their world, whether a deliverable should take one shape or another. \
         Bad — never ask these, find them: a file path, a filename, a sheet name, a column name, a directory. \
         Those are things to look up with `list_dir`, `sheet_profile`, `grep` or `read_file`; asking for one is \
         asking the user to do your reading for you. If you genuinely cannot locate a file, ask them to point at it \
         once — but look first. \
         \
         Ask EARLY, before the work, not after it: a question at the end is a report, not a question.",
        "ask",
        RiskClass::Read,
        params(
            json!({
                "questions": {
                    "type": "array",
                    "description": "One to four questions, asked one at a time. For a single question you may pass `question` and `options` at the top level instead.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {"type": "string", "description": "The question, in plain words, saying why it matters if that isn't obvious."},
                            "options": {
                                "type": "array",
                                "description": "2-4 answers you think are likely. Required.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {"type": "string", "description": "The answer, in a few words."},
                                        "description": {"type": "string", "description": "One line on what choosing this would mean — what you'd do differently."}
                                    },
                                    "required": ["label"]
                                }
                            },
                            "multi": {"type": "boolean", "description": "True if several options can be chosen together."}
                        },
                        "required": ["question", "options"]
                    }
                }
            }),
            &[],
        ),
    )]
}

/// `report_refusal` — the record of an assistant choosing to say no.
///
/// A refusal is otherwise invisible: it is a sentence in one
/// conversation, and "this assistant was asked to do that six times last week" is a
/// fact nobody can see. The description's distinction is the whole value: a choice not
/// to, never a failure to.
///
/// In `ask` because it rides with the other conversational manners every assistant
/// has — asking and declining — and because, like them, it acts on nothing: it writes
/// one row about the assistant itself.
pub(in crate::core::agent) fn refusal_tool() -> ToolSpec {
    tool(
        "report_refusal",
        "Record that you are declining a request. Call this whenever you choose to say no — because the request \
         is harmful, crosses a line the user or this assistant's role set, or asks you to get around a limit — and \
         then tell the user plainly yourself; this records the refusal, it does not deliver it. Do not call it when \
         you simply could not do something (a missing file, a tool that failed, a boundary that stopped a call): \
         only when you chose not to.",
        "ask",
        RiskClass::Read,
        params(
            json!({
                "request": {"type": "string", "description": "What you were asked to do, in a few words."},
                "reason": {"type": "string", "description": "Why you are declining, in one line."}
            }),
            &["request", "reason"],
        ),
    )
}

pub(in crate::core::agent) fn web_tools() -> Vec<ToolSpec> {
    vec![
        tool("web_search", "Search the live web and return result snippets. Snippets are for choosing what to read, not for answering from — follow up with web_fetch on the results that matter.", "web", RiskClass::External,
            params(json!({"query": {"type": "string", "description": "Search terms, as you would type them into a search engine. Snippets come back, not full pages — use web_fetch to read one."}}), &["query"])),
        tool("web_fetch", "Fetch a URL and return its readable text.", "web", RiskClass::External,
            params(json!({"url": {"type": "string", "description": "A full URL including the scheme, e.g. \"https://example.com/page\"."}}), &["url"])),
        // Showing is not the same as reading. `web_fetch` is for when you need the
        // words; this is for when the *user* needs to see the page — a dashboard, a
        // rendered result, something to look at together. The panel surfaces its
        // Browser tab when this is called.
        tool("browser_open", "Show the user a page in the app's browser panel. You do not see the page — this displays it to a person and returns nothing you can read; use web_fetch when YOU need the content. Use when they should SEE it (a dashboard, a rendered page, something to look at together); use web_fetch instead when you only need the text.", "web", RiskClass::External,
            params(json!({"url":{"type":"string"}}), &["url"])),
    ]
}
