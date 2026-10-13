use super::super::*;

/// The work board: items that outlive a turn, and the review that has to happen
/// before one counts as finished.
///
/// `board_review` is the load-bearing tool and the one the descriptions spend
/// their words on, because the failure it prevents is the one that happens by
/// default: an assistant finishing its own work and marking it done.
pub(in crate::core::agent) fn board_tools() -> Vec<ToolSpec> {
    vec![
        tool("board_add",
            "Put a work item on the project's board. Use this for work that outlives this turn or that someone \
             else will pick up — not as a progress list for your own turn, which is `todo_write`. Always write \
             `acceptance`: an item without it can only ever be reviewed on someone's opinion.",
            "board", RiskClass::WriteLocal,
            params(json!({
                "title": { "type": "string", "description": "One line, the work itself." },
                "detail": { "type": "string", "description": "What an assistant picking this up cold would need." },
                "acceptance": { "type": "string", "description": "What has to be true for this to be done. Checkable, not aspirational." }
            }), &["title", "acceptance"])),
        tool("board_list", "Read the project's board. Optionally filter to one state.", "board", RiskClass::Read,
            params(json!({ "state": { "type": "string", "enum": ["open", "in_progress", "blocked", "review", "done", "canceled"] } }), &[])),
        tool("board_assign", "Say who is doing an item.", "board", RiskClass::WriteLocal,
            params(json!({
                "id": { "type": "string" },
                "to": { "type": "string", "description": "The assistant or person taking it." }
            }), &["id", "to"])),
        tool("board_move",
            "Move an item between columns. Moving to `done` requires a passing review from an assistant other than \
             whoever put it up for review — you cannot mark your own work done, and trying will be refused.",
            "board", RiskClass::WriteLocal,
            params(json!({
                "id": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "in_progress", "blocked", "review", "done", "canceled"] }
            }), &["id", "state"])),
        tool("board_note", "Add a note to an item — what you tried, what you found, what is blocking it.", "board", RiskClass::WriteLocal,
            params(json!({ "id": { "type": "string" }, "text": { "type": "string" } }), &["id", "text"])),
        tool("board_review",
            "Record a verdict on an item someone else built, checked against its acceptance criteria. Evidence is \
             required: what you ran, what you read, what you saw. A verdict with no evidence is an opinion.",
            "board", RiskClass::WriteLocal,
            params(json!({
                "id": { "type": "string" },
                "pass": { "type": "boolean" },
                "evidence": { "type": "string", "description": "What you actually did to check. Not \"looks correct\"." }
            }), &["id", "pass", "evidence"])),
    ]
}

/// The scanner toolchain: find out what is installed, and keep the record of what
/// each check actually did.
///
/// `check_tool` is the one that matters. It does not merely answer a question —
/// it WRITES the coverage ledger as a side effect, because the failure this
/// capability exists to prevent is a check quietly disappearing between
/// discovering its tool is missing and writing the report. A model that never
/// mentions the gap has still not managed to hide it.
pub(in crate::core::agent) fn toolchain_tools() -> Vec<ToolSpec> {
    let catalog = crate::core::toolchain::names();
    vec![
        tool("check_tool", &format!(
            "Check whether a security scanner is installed on this machine, and get its version. \
             Call this BEFORE running any scan — never assume a scanner is present. Known tools: {catalog}. \
             A missing tool is recorded against your coverage automatically; you cannot skip it silently."),
            "toolchain", RiskClass::Read,
            params(json!({ "name": { "type": "string", "description": "The scanner to check for." } }), &["name"])),
        tool("request_tool",
            "Ask the user to install a scanner you need but can't find. Shows them the exact install command \
             to approve or decline. If they decline, carry on with a manual equivalent and say in your report \
             that the check was degraded — never drop it.",
            "toolchain", RiskClass::Exec,
            params(json!({
                "name": { "type": "string", "description": "The scanner to install." },
                "reason": { "type": "string", "description": "What you need it for, in one line the user can judge." }
            }), &["name", "reason"])),
        tool("record_coverage",
            "Record what actually happened to one check, for the Coverage note. Use it to amend a check you \
             worked around: if a tool was missing but you did the work by hand, that check is `degraded`, not skipped.",
            "toolchain", RiskClass::Read,
            params(json!({
                "check": { "type": "string", "description": "The check, named the same way each time (e.g. \"Secret sweep\")." },
                "status": { "type": "string", "enum": ["ran", "degraded", "skipped"] },
                "detail": { "type": "string", "description": "For `ran`, the tool and version. Otherwise, why — what is left uncovered." }
            }), &["check", "status", "detail"])),
        tool("coverage_note",
            "Render the Coverage note from what actually happened. End every security review with it — and use \
             what this returns verbatim rather than writing your own, because this one is built from the record, not memory.",
            "toolchain", RiskClass::Read, params(json!({}), &[])),
    ]
}

pub(in crate::core::agent) fn files_tools() -> Vec<ToolSpec> {
    let mut out = plain_file_tools();
    out.extend(sheet_tools());
    out.extend(office_tools());
    out
}

pub(in crate::core::agent) fn plain_file_tools() -> Vec<ToolSpec> {
    vec![
        tool("read_file", "Read a file's contents — including PDFs, Word, PowerPoint and spreadsheets, which come back as text. Optionally restrict to a line range (text files only).", "files", RiskClass::Read,
            params(json!({
                "path": {"type": "string", "description": "Path relative to the working directory, e.g. \"src/main.rs\". It must be a path you have actually seen — named by the user, or returned by list_dir or grep. Do not guess a plausible-looking one; list the folder instead."},
                "start_line": {"type": "integer", "description": "First line to return, counting from 1. Text files only."},
                "end_line": {"type": "integer", "description": "Last line to return, inclusive. Omit both to read the whole file."}
            }), &["path"])),
        tool("write_file", "Create a new file or overwrite an existing one with the given content.", "files", RiskClass::WriteLocal,
            params(json!({
                "path": {"type": "string", "description": "Path relative to the working directory."},
                "content": {"type": "string", "description": "The COMPLETE new contents of the file. This replaces everything already there. To add to a file, read it first and pass the old text plus the new — or use edit_file, which does not need the rest."}
            }), &["path","content"])),
        tool("edit_file", "Replace an exact substring in a file. old_str must match exactly once.", "files", RiskClass::WriteLocal,
            params(json!({
                "path": {"type": "string", "description": "Path relative to the working directory."},
                "old_str": {"type": "string", "description": "Text to replace, copied EXACTLY from the file including indentation and line breaks. It must appear exactly once: if the line you want occurs more than once, include the lines above or below it until the whole block is unique."},
                "new_str": {"type": "string", "description": "What replaces it. Everything outside old_str is left untouched."}
            }), &["path","old_str","new_str"])),
        tool("list_dir", "List the entries of a directory.", "files", RiskClass::Read,
            params(json!({"path": {"type": "string", "description": "Directory relative to the working directory. \".\" is the working directory itself."}}), &["path"])),
    ]
}
