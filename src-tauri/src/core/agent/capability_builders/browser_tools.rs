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
    let r#ref = json!({"type": "string", "description": "The ref from your latest browser_snapshot, e.g. \"e4\"."});
    let snapshot_id = json!({"type": "integer", "description": "The snapshot_id that snapshot gave you."});
    vec![
        tool("browser_navigate",
            "Open a web page in your own browser — a real Chrome that stays signed in to the sites the user signed it into, \
             and that the user can watch. Use it to act on a site, or to read a page that needs a sign-in or JavaScript. \
             Returns the page's title and readable text; answer from that. (For a public page you only need to read, \
             web_fetch is lighter. To show the user a page, use browser_open.)",
            "browser", RiskClass::External,
            params(json!({"url": {"type": "string", "description": "The full address, including https://."}}), &["url"])),
        tool("browser_read",
            "Read the page open in your browser now, without going anywhere. Use it after clicking or submitting something, \
             to find out what the page says.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_snapshot",
            "List what you can act on in the current page — fields, buttons, links, checkboxes, dropdowns — each with a ref \
             such as e4, its label and its current value, plus a snapshot_id. Call it BEFORE clicking or typing, and use only \
             the refs it returns. If an action says your refs are out of date, the page changed: call it again.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_click",
            "Click a button, link, checkbox, radio option or tab, by its ref from your latest snapshot.",
            "browser", RiskClass::External,
            params(json!({"ref": r#ref, "snapshot_id": snapshot_id}), &["ref", "snapshot_id"])),
        tool("browser_type",
            "Put text into a field, replacing what is already there. For a dropdown, give the option's label. Set submit to \
             press Enter afterwards. Never use this for a password or a one-time code — use browser_request_secret.",
            "browser", RiskClass::External,
            params(json!({
                "ref": r#ref, "snapshot_id": snapshot_id,
                "text": {"type": "string", "description": "What to put in the field."},
                "submit": {"type": "boolean", "description": "Press Enter after typing, e.g. to run a search."}
            }), &["ref", "snapshot_id", "text"])),
        tool("browser_key",
            "Press a key — Enter, Tab, Escape, ArrowDown, Backspace, or a combination such as Control+A — on a field by its \
             ref, or on the page when you give no ref.",
            "browser", RiskClass::External,
            params(json!({
                "key": {"type": "string", "description": "The key name, e.g. \"Enter\"."},
                "ref": r#ref, "snapshot_id": snapshot_id
            }), &["key"])),
        tool("browser_scroll",
            "Scroll the page to bring more of it into view, then take a new snapshot.",
            "browser", RiskClass::WriteLocal,
            params(json!({
                "direction": {"type": "string", "enum": ["down", "up"]},
                "pages": {"type": "integer", "description": "How far, in screens. Default 1."}
            }), &[])),
        tool("browser_request_help",
            "Hand your browser to the user for something only they can do: sign in, pass a two-factor check or a CAPTCHA. \
             Say exactly what they need to do. They take control, do it, and hand it back, and you carry on from there. \
             Calling this IS asking: writing 'please sign in and let me know' in your answer reaches nobody and leaves the \
             page where it is. Never ask the user for a password or a code in the conversation instead.",
            "browser", RiskClass::Read,
            params(json!({"reason": {"type": "string", "description": "What the user needs to do, in one sentence, e.g. \"This page wants the code sent to your phone.\""}}), &["reason"])),
        tool("browser_request_secret",
            "Ask the user for ONE value you must not see — a password, a one-time code, a card number — for the field with \
             this ref. They type it into a masked box that goes straight into the page; you are only told that it arrived. \
             Prefer this to handing over the whole browser when one field is all that's needed. Submitting the form \
             afterwards is up to you.",
            "browser", RiskClass::Read,
            params(json!({
                "label": {"type": "string", "description": "What you need, in a few words, e.g. \"your account password\"."},
                "ref": r#ref, "snapshot_id": snapshot_id
            }), &["label", "ref", "snapshot_id"])),
        tool("browser_console",
            "What the page reported on its own since you last looked — console errors and warnings, uncaught exceptions, \
             and requests that failed or answered with an error — and the browser and system it ran on. For a bug report. \
             Each look takes what's there, so look once before the step you're checking and again after it.",
            "browser", RiskClass::Read, params(json!({}), &[])),
        tool("browser_screenshot",
            "Keep a picture of the page as it looks right now — the moment a bug shows, say. It appears in the conversation \
             and stays with it. A page you navigate to is kept already; this is for what happens after.",
            "browser", RiskClass::Read,
            params(json!({"why": {"type": "string", "description": "What the picture shows, in a few words, e.g. \"the error after Pay\"."}}), &[])),
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
