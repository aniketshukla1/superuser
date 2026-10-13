use super::super::*;

/// Spreadsheets, as documents to work IN rather than files to regenerate.
///
/// `write_file` can produce a .csv, and nothing more: an .xlsx is a zip of XML, so
/// an assistant asked for a real workbook could only ever hand back a flat file with
/// the formatting, sheets and formulas missing. These edit the workbook itself —
/// and edit it cell by cell, so the user's own rows and notes survive a change the
/// agent makes beside them.
///
/// Part of `files`, deliberately, rather than a capability of its own. A separate
/// one would have to be granted per persona, so an assistant created tomorrow would
/// silently be unable to touch a spreadsheet — and the create-assistant API only
/// accepts six tool ids, so it could not have been granted there at all. The
/// permission being asked for is the same one either way: read and write inside
/// the workspace. A file tool that only understands text is a limitation, not a
/// safety boundary.
pub(in crate::core::agent) fn sheet_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "sheet_sources",
            "Where a workbook's values came from — the receipt, email or page each cell was read from. Use it to answer \"where did this number come from?\" instead of guessing.",
            "files",
            RiskClass::Read,
            params(json!({ "path": {"type": "string"} }), &["path"]),
        ),
        tool(
            "sheet_read",
            "Read a spreadsheet's cells as rows. Use before editing, so you change the right cells.",
            "sheets",
            RiskClass::Read,
            params(
                json!({
                    "path": {"type": "string", "description": "Workbook path, relative to the workspace."},
                    "sheet": {"type": "string", "description": "Sheet name. Defaults to the first."}
                }),
                &["path"],
            ),
        ),
        // A chart is drawn in the CONVERSATION, and saved only if asked.
        //
        // It used to be a written artifact first: `path` was required, so every chart left
        // a `.chart.json` behind whether or not anyone wanted one — eleven of them in a
        // single session here, none requested. The transcript never needed the file; it
        // reads the spec off this call. Drawing and keeping were separate things welded
        // together, and the weld is what filled the panel with clutter.
        //
        // Still a spec rather than code: a chart is JSON, checked and drawn, and nothing
        // here executes.
        tool(
            "chart",
            "Draw a chart from data you already have. It is drawn in the conversation immediately. \
             Pass `path` ONLY if the user asked to save or keep it — otherwise leave it out and the chart is just shown. \
             Prefer this over describing numbers in prose when a shape (trend, share, comparison) is the point. \
             The numbers in `data` MUST be values you actually read from a file or a tool result in this conversation — never remembered, rounded or invented. \
             If you have not read them, read them first; a chart of plausible-looking numbers is worse than no chart, because it looks like evidence. \
             Spec: {\"type\": bar|line|area|scatter|pie, \"title\": str, \"note\": str (say what the chart SHOWS — the finding, not a caption), \
             \"x\": {\"field\": str}, \"y\": {\"field\": str}, \"series\": [{\"field\": str, \"label\": str}] (optional, for several lines/bars), \
             \"data\": [{...}] (rows of plain values)}. For pie, `x` names the label column and `y` the value column.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string", "description": "Omit unless the user asked to SAVE the chart. When given, also writes it there, ending .chart.json."},
                    "spec": {"type": "object", "description": "The chart spec, as described above."},
                    // Not decoration. This is checked: name a file or URL here and it must be
                    // one this run actually opened, or the call is refused. Prose is accepted
                    // when prose is the truth — see core::gates::grounding.
                    "source": {"type": "string", "description": "Where the numbers came from. Best: name a .csv, .tsv or .json file in your folder and leave `data` out — its rows are read for you, so the picture is the file rather than your retyping of it. Otherwise say where they came from (a page you fetched, the figures in the user's message). Checked against what this run has actually read; naming a file you have not opened is refused."}
                }),
                &["spec", "source"],
            ),
        ),
        // A summary the agent composes: the facts, and what they mean.
        //
        // The shape a result wants when it is neither a chart nor a table — a runway
        // date, a variance total, a scan's headline. Prose can say all of it and says
        // it worse: the numbers land in a sentence where they cannot be scanned or
        // found again three turns later.
        //
        // Read-only, and conversation-only. No path: a card is a way of SAYING
        // something, not a deliverable, and deliverables already have `write_file`.
        // No buttons either — this app has considered ways for the agent to make
        // things happen (`ask_user`, `propose_plan`, the approval card), and a second
        // one hiding inside a summary block would be an authority nobody reviewed.
        tool(
            "card",
            "Summarise a result in the conversation: a title, the few facts that matter, and what they mean. \
             Use it for an answer whose shape is 'here is the figure and here is what it tells you' — a runway date, \
             a variance total, the headline of a scan. Use `table` when there are rows, `chart` when the shape is the point. \
             Keep it to the facts a person would repeat: two to five. A card of twelve is a table. \
             The values MUST be ones you actually read this run — never remembered, rounded or invented. \
             Spec: {\"title\": str, \"note\": str (what it MEANS — the finding, not a caption), \
             \"tone\": neutral|good|warn|bad (optional, colours the edge only), \
             \"kind\": facts|checklist|metrics|notice (optional, default facts), \
             \"facts\": [{\"label\": str, \"value\": str, \"emphasis\": bool (at most one — the figure the card is about), \
             \"done\": bool (checklist only: whether that one passed), \"delta\": str (metrics only: how it moved, e.g. \"+12% on last week\")}]}. \
             A **checklist** is what was checked and what passed — every entry needs `done`, because an item whose state \
             is unsaid is the one a person assumes passed. **metrics** is a row of figures with how each moved. \
             A **notice** is one thing worth saying on its own: it needs only `note` and a `tone`, and carries no facts.",
            "files",
            RiskClass::Read,
            params(
                json!({
                    "spec": {"type": "object", "description": "The card spec, as described above."},
                    "source": {"type": "string", "description": "Where these facts came from — the file you read, the page you fetched, or 'the figures in the user's message'. Checked against what this run has actually read."}
                }),
                &["spec", "source"],
            ),
        ),
        // A table, as DATA rather than as markdown.
        //
        // Prose tables work — remark-gfm renders them — and they are the right answer
        // for three rows of text. They stop being the right answer the moment the table
        // is a result: markdown cannot say "this column is a number", so nothing can
        // right-align it, sort by it, or hand it to a spreadsheet. Emitting the rows
        // instead of a picture of the rows is what makes those possible, and it is the
        // same trade `chart` already makes.
        tool(
            "table",
            "Show tabular data in the conversation, as data rather than as markdown. \
             Use this when the table IS a result — figures you read, rows you found — and prose markdown when it is a handful of words. \
             Columns declare their kind, so numbers align and sort as numbers and the user can copy the whole thing as CSV. \
             The values MUST be ones you actually read from a file or a tool result in this conversation — never remembered, rounded or invented. \
             Pass `path` ONLY if the user asked to save it. \
             Spec: {\"title\": str, \"note\": str (what the table SHOWS — the finding, not a caption), \
             \"columns\": [{\"field\": str, \"label\": str, \"kind\": text|number (default text)}], \
             \"rows\": [{...}] (plain values, keyed by column field)}.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string", "description": "Omit unless the user asked to SAVE it. When given, writes a CSV there."},
                    "spec": {"type": "object", "description": "The table spec, as described above."},
                    "source": {"type": "string", "description": "Where the rows came from. Best: name a .csv, .tsv or .json file in your folder and leave `rows` out — they are read for you, rather than retyped. Otherwise say where they came from. Checked against what this run has actually read."}
                }),
                &["spec", "source"],
            ),
        ),
        // Reading a sheet gives you cells; this gives you what the columns ARE. An agent
        // asked "what does this data say?" otherwise pulls thousands of rows into context to
        // work out that column C is a date — a question the file can answer about itself.
        tool(
            "sheet_profile",
            "Summarise a spreadsheet's columns without reading every row: for each column, its name, inferred type, how many values are filled, how many are distinct, and min/max or the commonest values. \
             Use this FIRST when asked what a file contains or to chart it — it costs a fraction of sheet_read and tells you which columns are worth plotting.",
            "sheets",
            RiskClass::Read,
            params(
                json!({
                    "path": {"type": "string", "description": "Workbook or CSV path, relative to the workspace."},
                    "sheet": {"type": "string", "description": "Sheet name. Defaults to the first."}
                }),
                &["path"],
            ),
        ),
        tool(
            "sheet_new",
            "Create a new .xlsx workbook with the given sheet names. Fails if the file already exists.",
            "sheets",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "sheets": {"type": "array", "items": {"type": "string"}, "description": "Sheet names, in order."}
                }),
                &["path"],
            ),
        ),
        tool(
            "sheet_write",
            "Set specific cells in a workbook. Everything you do not name is left exactly as it is — prefer this over rewriting a file. A value starting with = is a formula.",
            "sheets",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "sheet": {"type": "string", "description": "Sheet name. Defaults to the first."},
                    "cells": {
                        "type": "array",
                        "description": "Cells to set, e.g. [{\"cell\":\"B7\",\"value\":\"12\",\"source\":\"receipts/notion-aug.pdf\",\"quote\":\"Amount: 12.00 USD\"}]. Send every cell you mean to change in ONE call: each call rewrites the whole workbook, so 500 cells together take 23ms and the same 500 one at a time take about 15 seconds. Give `source` whenever the value came from somewhere the user could check — a receipt, an email, a page. It is recorded beside the workbook, so a number can be traced back later.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "cell": {"type": "string"},
                                "value": {"type": "string"},
                                "source": {"type": "string", "description": "File path, URL, or message the value was read from."},
                                "quote": {"type": "string", "description": "The line it was read from, so the claim can be checked."}
                            },
                            "required": ["cell", "value"]
                        }
                    }
                }),
                &["path", "cells"],
            ),
        ),
    ]
}
