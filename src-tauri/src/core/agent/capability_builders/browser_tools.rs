use super::super::*;

/// The assistant's own browser — `core::agentbrowser`.
///
/// Each description carries a lesson: snapshot
/// before acting and never invent a ref; a stale ref means snapshot again; calling
/// `request_help` IS asking, and a sentence in the answer reaches nobody; one secret value
/// goes through `request_secret` and never through the conversation.
///
/// Not granted by any built-in persona's manifest. An assistant that can use the web is
/// given these when the user switches the browser on in Settings (`commands::run_agent_inner`);
/// `browser` exists as a capability so a persona can declare it explicitly too.
pub fn browser_tools() -> Vec<ToolSpec> {
    let r#ref = json!({"type": "string", "description": "The ref your latest browser_snapshot gave the element, e.g. \"e4\"."});
    let snapshot_id = json!({"type": "integer", "description": "The snapshot_id returned with that snapshot."});
    vec![
        tool("browser_navigate",
            "Load a web page in your own browser: a real Chrome, kept signed in to the sites the user has signed it \
             into, which the user can watch. Use it to do something on a site, or to read a page that needs a sign-in or \
             runs JavaScript. You get back the page's title and readable text to answer from. (web_fetch is lighter for a \
             public page you only need to read; browser_open shows a page to the user.)",
            "browser", RiskClass::External,
            params(json!({"url": {"type": "string", "description": "The complete address, with https://."}}), &["url"])),
        tool("browser_read",
            "Get the text of the page your browser is on now, without loading anything new. Useful after a click or a \
             submit, to see what the page says.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_snapshot",
            "List the things on the current page you can act on (fields, buttons, links, checkboxes, dropdowns), each \
             with a ref like e4, its label and its current value, plus a snapshot_id. Take one BEFORE you click or type, and \
             act only on refs from it. If an action says your refs are out of date, the page has moved on: snapshot again.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_click",
            "Click something by the ref your latest snapshot gave it: a button, link, checkbox, radio option or tab.",
            "browser", RiskClass::External,
            params(json!({"ref": r#ref, "snapshot_id": snapshot_id}), &["ref", "snapshot_id"])),
        tool("browser_type",
            "Fill a field with text, replacing its current contents; for a dropdown, give the label of the option. Set \
             submit to press Enter afterwards. Passwords and one-time codes never go through this tool: use \
             browser_request_secret.",
            "browser", RiskClass::External,
            params(json!({
                "ref": r#ref, "snapshot_id": snapshot_id,
                "text": {"type": "string", "description": "The text for the field."},
                "submit": {"type": "boolean", "description": "Also press Enter, e.g. to start a search."}
            }), &["ref", "snapshot_id", "text"])),
        tool("browser_key",
            "Press a key (Enter, Tab, Escape, ArrowDown, Backspace) or a combination such as Control+A, on the field with \
             this ref, or on the page itself when no ref is given.",
            "browser", RiskClass::External,
            params(json!({
                "key": {"type": "string", "description": "Which key, e.g. \"Enter\"."},
                "ref": r#ref, "snapshot_id": snapshot_id
            }), &["key"])),
        tool("browser_scroll",
            "Scroll to bring more of the page into view; snapshot again afterwards.",
            "browser", RiskClass::WriteLocal,
            params(json!({
                "direction": {"type": "string", "enum": ["down", "up"]},
                "pages": {"type": "integer", "description": "Distance in screens. 1 if left out."}
            }), &[])),
        tool("browser_request_help",
            "Give the user your browser for a step only they can take: signing in, a two-factor check, a CAPTCHA. Tell \
             them precisely what to do. They take over, finish, and give it back, and you continue. This call is the \
             request itself: a line in your reply asking them to sign in is seen by no one and changes nothing on the \
             page. Never ask for a password or code in the conversation as a substitute.",
            "browser", RiskClass::Read,
            params(json!({"reason": {"type": "string", "description": "One sentence on what the user has to do, e.g. \"The site is asking for the code it texted you.\""}}), &["reason"])),
        tool("browser_request_secret",
            "Get ONE value from the user that you must never see (a password, a one-time code, a card number) for the \
             field with this ref. The user types it into a hidden input that fills the page directly, and you only learn \
             that it arrived. When a single field is all you need, use this rather than handing over the whole browser. \
             You still submit the form yourself.",
            "browser", RiskClass::Read,
            params(json!({
                "label": {"type": "string", "description": "Names the value for the user, e.g. \"your account password\"."},
                "ref": r#ref, "snapshot_id": snapshot_id
            }), &["label", "ref", "snapshot_id"])),
        tool("browser_console",
            "Everything the page logged by itself since your last look (console errors and warnings, uncaught \
             exceptions, requests that failed or returned an error), plus the browser and system it ran on. Meant for bug \
             reports. Each call empties what it reads, so look once before the step you're testing and once after.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_screenshot",
            "Save a picture of the page exactly as it is now, for example the moment a bug appears. It shows in the \
             conversation and stays there. Pages you navigate to are captured already; use this for what happens after.",
            "browser", RiskClass::Read,
            params(json!({"why": {"type": "string", "description": "A few words on what it shows, e.g. \"the error after Pay\"."}}), &[])),
    ]
}

/// Whether a tool is one of the browser's.
pub fn is_browser_tool(name: &str) -> bool {
    name.starts_with("browser_") && name != "browser_open"
}

/// Reading back tool output that was truncated out of the window.
///
/// Named `recall_output`, not `recall`, and the distance from `recall_memory` is
/// the point: one searches the user's notes, the other returns bytes this run
/// already produced. HARNESS-RESEARCH-2026 §2 proposes `recall`, written without
/// knowing `recall_memory` was already in the catalogue — and this repo's own
/// measurements are of small models picking the wrong tool from a list, so two
/// tools called `recall*` with unrelated jobs is a fight worth not having.
///
/// The description leads with the handle because that is what the model has: it
/// read the notice in a truncated result and is holding an id.
pub(in crate::core::agent) fn recall_output_tools() -> Vec<ToolSpec> {
    vec![tool("recall_output",
        "Read back tool output that was truncated out of the transcript. Pass the id from a \
         \"… [N characters truncated …] …\" notice to get the whole original result. Optionally \
         pass `from` and `len` to read one window of it instead. This returns exactly what was \
         produced at the time, so prefer it over re-running a command or re-reading a file.",
        "recall", RiskClass::Read,
        params(json!({
            "id": {"type": "string", "description": "The handle from the truncation notice."},
            "from": {"type": "integer", "description": "Start character offset. Omit for the whole result."},
            "len": {"type": "integer", "description": "How many characters from `from`."}
        }), &["id"]))]
}

pub(in crate::core::agent) fn recall_memory_tools() -> Vec<ToolSpec> {
    vec![tool("recall_memory", "Search the user's big-brain memory for relevant context.", "recall_memory", RiskClass::Read,
        params(json!({"query":{"type":"string"},"limit":{"type":"integer"}}), &["query"]))]
}

pub(in crate::core::agent) fn computer_tools() -> Vec<ToolSpec> {
    vec![tool(
        "computer_use",
        "Drive the desktop (open apps, click, type) to accomplish a described task. For short, \
         visually obvious steps only — it works from screenshots, so anything buried in a menu or \
         needing many steps is better done with a file, a shell command, or the app's own tool.",
        "computer",
        RiskClass::Exec,
        params(json!({"task": {"type": "string", "description": "One concrete outcome, stated the way you would tell a person watching the screen — \"open System Settings and turn on Night Shift\", not \"configure display preferences\"."}}), &["task"]),
    )]
}

pub(in crate::core::agent) fn messaging_tools() -> Vec<ToolSpec> {
    vec![tool("send_message", "Send a message on the user's behalf. `target` is channel:address (e.g. `slack:#ops`, `telegram:12345`). Consequential — it will be approval-gated.", "messaging", RiskClass::External,
        params(json!({"target":{"type":"string"},"text":{"type":"string"}}), &["target","text"]))]
}

pub(in crate::core::agent) fn subagent_tools() -> Vec<ToolSpec> {
    vec![tool(
        "spawn_subagent",
        "Delegate self-contained subtasks to fresh subagents and get back only their conclusions. \
         Pass several tasks to run them at the same time. Use this when work would produce a lot of \
         intermediate output you don't need, or when independent things can happen in parallel. \
         A subagent cannot see this conversation and cannot ask questions, so state every path, id \
         and decision it needs. Don't use it for steps whose intermediate results you need yourself.",
        "subagent",
        RiskClass::Read,
        params(
            json!({
                "tasks": {
                    "type": "array",
                    "description": "One entry per independent subtask.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "task": { "type": "string", "description": "The complete, self-contained instruction." },
                            "type": { "type": "string", "description": "Subagent type: 'general-purpose' (default) or 'explore' for read-only research." }
                        },
                        "required": ["task"]
                    }
                }
            }),
            &["tasks"],
        ),
    )]
}
