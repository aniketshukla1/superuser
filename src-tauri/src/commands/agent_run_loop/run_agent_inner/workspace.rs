/// The system prompt's workspace half: AGENTS.md, how documents work here, the clock
/// (volatile) and the private working folder.
pub(super) fn workspace_prompt(
    system: &mut String,
    volatile: &mut String,
    specs: &[crate::core::agent::ToolSpec],
    sub: &Option<crate::core::subagent::SubagentType>,
    working_dir: &std::path::Path,
) {
    // AGENTS.md — the assistant's durable instructions, living in the workspace where
    // the user can see and edit them, rather than in a settings dialog or a persona
    // manifest they have no way to reach. This is the point of the file: editing it
    // edits what the agent is, and the edit is an ordinary file edit in the panel.
    //
    // After the persona and the profile, so it can override both — it is the most
    // specific instruction the user has given, and the most recent. Not given to
    // subagents, for the same reason the persona is not: a delegate runs under its
    // type's instructions.
    if sub.is_none() {
        if let Ok(text) = std::fs::read_to_string(working_dir.join("AGENTS.md")) {
            let text = text.trim();
            // An empty file is the user clearing it, which should read as "no extra
            // instructions" rather than as an empty heading in the prompt.
            if !text.is_empty() {
                // Capped: this is a file the user can paste anything into, and it
                // goes into every turn of every conversation in this workspace.
                const MAX: usize = 8000;
                let body = if text.len() > MAX {
                    let mut cut = MAX;
                    while cut > 0 && !text.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    format!("{}\n\n[AGENTS.md truncated at {MAX} characters]", &text[..cut])
                } else {
                    text.to_string()
                };
                system.push_str(&format!(
                    "\n\n## Instructions from this workspace (AGENTS.md)\n\n\
                     The user maintains this file in the workspace and can edit it at any time. \
                     Treat it as their standing instructions for work done here, taking precedence \
                     over your general defaults where the two differ.\n\n{body}"
                ));
            }
        }
    }

    // Documents are first-class here, and a model has no way to know that. Every
    // provider's default habit is to reach for `write_file` with a document name —
    // which produced the 58-byte `monthly_budget.xlsx` of prose that started this.
    // `write_file` now refuses those extensions, but a refusal after the fact is a
    // wasted turn; saying so up front is the cheaper half of the same fix.
    //
    // Conditional on the tools actually being present, like the subagent menu
    // below: a persona without the `files` capability shouldn't be told about
    // tools it cannot call.
    if specs.iter().any(|s| s.name == "sheet_new") {
        system.push_str(
            "\n\n## Documents are real files here\n\n\
             Spreadsheets, Word documents and slide decks are produced with their own tools, \
             not by writing text to a file with that extension — `write_file` will refuse a \
             .xlsx/.docx/.pptx path, because bytes written that way make a file no application \
             can open.\n\n\
             **Creating one:** `sheet_new` for a spreadsheet, `doc_write` for a Word document, \
             `slides_write` for a deck.\n\n\
             **Changing one that already exists — read it first, then edit the parts you mean:** \
             `sheet_read` then `sheet_write` for cells; `doc_read` then `doc_edit` for paragraphs; \
             `slides_read` then `slides_edit` for the text on slides. These leave everything you \
             did not name exactly as it was — the styles, tables, images, layout and design. \
             `doc_write` and `slides_write` REPLACE the whole file, so using one to make a small \
             change destroys work the user cannot get back. Reach for them only to create \
             something new.\n\n\
             Slides, shapes and paragraphs are numbered from 1, exactly as the read tools list \
             them. `read_file` also extracts plain text from PDFs and Office documents when you \
             only need to know what one says.",
        );
    }
    // The current date and time. Without it a model answers "next Friday" and
    // "tomorrow at 9" from whenever its training data ended — measured: asked to
    // schedule something for 9am, one invented a 2023 timestamp.
    // Rounded to the HOUR, and in the volatile half.
    //
    // At minute resolution this line changed on every request that crossed a minute
    // boundary, which for a multi-step turn is most of them — a per-request
    // cache-buster for a fact nothing in the product needs to the minute. An hour is
    // as much precision as "what time is it roughly" has ever wanted here.
    volatile.push_str(&format!("\n\nRight now it is {}.", chrono::Local::now().format("%A %-d %B %Y, around %-I %p (%Z)")));
    system.push_str(&format!(
        "\n\nYour private working folder is: {}\nWrite any files or deliverables there using relative paths. When you're done, give a short summary and note what you produced.",
        working_dir.to_string_lossy()
    ));
    system.push_str(&crate::core::artifacts::prompt_note());
}
