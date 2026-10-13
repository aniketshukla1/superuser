use super::*;

pub(super) fn read_file_tools(root: &Path, _todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();
    let rel = || args["path"].as_str().unwrap_or("").to_string();

    let out = match name {
        "read_file" => {
            let full = root.join(rel());
            // A document is a file the user would say they can read, so the tool that
            // reads files should read it. `coder::read` returns raw bytes as text,
            // which for a .pdf or .docx is a screenful of noise the model then tries
            // to reason about — while `files::extract` (already here, already used by
            // the attachment path) returns the actual words. The extraction existed;
            // the read tool simply never called it.
            let ext = full.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "pdf" | "docx" | "pptx" | "xlsx" | "xlsm" | "xls" | "ods") {
                return Some(match crate::core::files::extract(&full.to_string_lossy()) {
                    Ok(e) => match (e.text, e.note) {
                        (Some(t), _) if !t.trim().is_empty() => t,
                        // A scanned PDF has no text to give. Saying so is the useful
                        // answer; returning an empty string reads as an empty file.
                        (_, Some(note)) => format!("(no text extracted: {note})"),
                        _ => "(no text in this document)".to_string(),
                    },
                    Err(err) => format!("error: {err}"),
                });
            }
            match coder::read(&root_s, &full.to_string_lossy()) {
                Ok(content) => match (args["start_line"].as_u64(), args["end_line"].as_u64()) {
                    (Some(s), Some(e)) if e >= s => content
                        .lines()
                        .skip(s.saturating_sub(1) as usize)
                        .take((e - s + 1) as usize)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => content,
                },
                Err(e) => format!("error: {e}"),
            }
        }
        "write_file" => {
            let r = rel();
            let content = args["content"].as_str().unwrap_or("");
            if r.trim().is_empty() || r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            // A .xlsx is a zip archive, not a text file. Writing prose to that name
            // produces something Excel refuses to open — and the failure is silent,
            // because `write_file` reports success and the file exists at the right
            // path with the right extension. Seen in the wild: a 58-byte
            // `monthly_budget.xlsx` containing "Sheet created for monthly budget."
            // The authoring tools already exist; name the right one and refuse.
            if let Some(refusal) = document_write_refusal(&r) {
                return Some(refusal);
            }
            // Resolve through the Coder's checker rather than `root.join(&r)`:
            // join *replaces* the root when `r` is absolute, and it happily walks
            // a symlink out of the workspace. `read_file` and `edit_file` have
            // always gone through this; this one didn't.
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            if let Some(parent) = full.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&full, content) {
                Ok(_) => format!("wrote {r} ({} bytes)", content.len()),
                Err(e) => format!("error: {e}"),
            }
        }
        // Read a document's paragraphs, numbered as `doc_edit` expects.
        "doc_read" => {
            let full = root.join(rel());
            return Some(match crate::core::docedit::read_paragraphs(&full) {
                Ok(paras) => {
                    let mut out = String::new();
                    for (i, t) in paras.iter().enumerate() {
                        // Numbered from 1, like the document itself and like anyone
                        // describing it. See `deck_edits_are_numbered_from_one` for
                        // what 0-based numbering cost when a model echoed it back.
                        out.push_str(&format!("[{}] {}\n", i + 1, t.replace('\n', " ⏎ ")));
                    }
                    if out.is_empty() { "this document has no paragraphs".into() } else { out }
                }
                Err(e) => format!("error: {e}"),
            });
        }
        // Change the words in a document without rebuilding it.
        "doc_edit" => {
            let r = rel();
            if r.trim().is_empty() || r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            let mut bad: Option<String> = None;
            let edits: Vec<(usize, String)> = args["edits"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|e| {
                            let n = e["paragraph"].as_u64()?;
                            if n == 0 {
                                bad = Some(
                                    "error: paragraphs are numbered from 1, as `doc_read` lists them".into(),
                                );
                                return None;
                            }
                            Some((n as usize - 1, e["text"].as_str()?.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(msg) = bad {
                return Some(msg);
            }
            if edits.is_empty() {
                return Some("error: no edits given — each needs a paragraph number and text".into());
            }
            return Some(match crate::core::docedit::write_paragraphs(&full, &edits) {
                Ok(n) => format!("changed {n} paragraph(s) in {r}"),
                Err(e) => format!("error: {e}"),
            });
        }
        // Read a deck's text, with the indexes `slides_edit` takes.
        "slides_read" => {
            let full = root.join(rel());
            return Some(match crate::core::pptedit::read_slides(&full) {
                Ok(slides) => {
                    let mut out = String::new();
                    for (i, shapes) in slides.iter().enumerate() {
                        // Numbered from 1, like PowerPoint and like anyone describing
                        // a deck. These were 0-based, and it cost a real edit: asked
                        // to change "slide 2", the model wrote to index 2 — the third
                        // slide — and was self-consistent with this listing while
                        // being wrong about the deck. The listing and the tool now
                        // count the way the person asking counts.
                        out.push_str(&format!("Slide {}:\n", i + 1));
                        for (j, t) in shapes.iter().enumerate() {
                            // One line each: what the model needs to pick a shape is
                            // the text, not the deck.
                            let one = t.replace('\n', " ⏎ ");
                            out.push_str(&format!("  [{}] {one}\n", j + 1));
                        }
                    }
                    if out.is_empty() { "this deck has no slides".into() } else { out }
                }
                Err(e) => format!("error: {e}"),
            });
        }
        _ => unreachable!("routed here only for its own tools"),
    };
    Some(out)
}
