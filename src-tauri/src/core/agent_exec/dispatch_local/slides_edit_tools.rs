use super::*;

pub(super) fn slides_edit_tools(root: &Path, _todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();
    let rel = || args["path"].as_str().unwrap_or("").to_string();

    let out = match name {
        // Change the words in a deck without rebuilding it.
        "slides_edit" => {
            let r = rel();
            if r.trim().is_empty() || r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            // 1-based on the way in, matching `slides_read` and how decks are
            // numbered everywhere else; 0-based inside, because they are array
            // indices. A 0 is a caller still thinking in offsets — refused rather
            // than silently treated as slide 1.
            let mut bad: Option<String> = None;
            let edits: Vec<(usize, usize, String)> = args["edits"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|e| {
                            let slide = e["slide"].as_u64()?;
                            let shape = e["shape"].as_u64()?;
                            if slide == 0 || shape == 0 {
                                bad = Some("error: slides and shapes are numbered from 1, as `slides_read` lists them".into());
                                return None;
                            }
                            Some((
                                slide as usize - 1,
                                shape as usize - 1,
                                e["text"].as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(msg) = bad {
                return Some(msg);
            }
            if edits.is_empty() {
                return Some("error: no edits given — each needs slide, shape and text".into());
            }
            return Some(match crate::core::pptedit::write_slides(&full, &edits) {
                Ok(n) => format!("changed {n} shape(s) in {r}"),
                Err(e) => format!("error: {e}"),
            });
        }
        "doc_write" | "slides_write" => {
            let r = rel();
            if r.trim().is_empty() || r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            if name == "doc_write" {
                let paragraphs: Vec<String> = args["paragraphs"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if paragraphs.is_empty() {
                    return Some("error: doc_write needs at least one paragraph".into());
                }
                match crate::core::docwrite::write_docx(&full, args["title"].as_str(), &paragraphs) {
                    Ok(()) => format!("wrote {r} ({} paragraph(s))", paragraphs.len()),
                    Err(e) => format!("error: {e}"),
                }
            } else {
                let slides: Vec<crate::core::docwrite::Slide> = args["slides"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| {
                                Some(crate::core::docwrite::Slide {
                                    title: v["title"].as_str()?.to_string(),
                                    bullets: v["bullets"]
                                        .as_array()
                                        .map(|b| b.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                        .unwrap_or_default(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                match crate::core::docwrite::write_pptx(&full, &slides) {
                    Ok(()) => format!("wrote {r} ({} slide(s))", slides.len()),
                    Err(e) => format!("error: {e}"),
                }
            }
        }
        // A card is drawn in the conversation and nowhere else. Nothing to write, so
        // nothing here touches disk: the whole tool is a validation and a receipt.
        "ui_component" => {
            let title = args["title"].as_str().unwrap_or("").trim().to_string();
            let html = args["html"].as_str().unwrap_or("");
            if let Some(problem) = crate::core::genui::problem(&title, html) {
                return Some(format!("error: {problem}"));
            }
            return Some(format!(
                "“{title}” is drawn in the conversation. Say what it shows in your answer as well — somebody reading \
                 this later may have only your words."
            ));
        }
        "card" => {
            let kind = args["spec"]["kind"].as_str().unwrap_or("facts").trim();
            if !["facts", "checklist", "metrics", "notice"].contains(&kind) {
                return Some("error: card `kind` must be one of facts, checklist, metrics, notice".into());
            }
            let facts = args["spec"]["facts"].as_array().cloned().unwrap_or_default();
            // A notice IS its sentence: a warning that has to carry three figures to be
            // allowed on screen is not a warning.
            if kind == "notice" {
                let note = args["spec"]["note"].as_str().unwrap_or("").trim();
                if note.is_empty() {
                    return Some("error: a notice card needs a `note` — the thing you are telling them.".into());
                }
                return Some("shown in the conversation as a notice. Don't repeat it in prose.".to_string());
            }
            if facts.is_empty() {
                return Some(format!(
                    "error: a {kind} card needs `facts` — {}",
                    match kind {
                        "checklist" => "one entry per thing checked, each with `done` true or false",
                        "metrics" => "one entry per figure, each with an optional `delta` saying how it moved",
                        _ => "a non-empty array of {label, value}",
                    }
                ));
            }
            if kind == "checklist" && facts.iter().any(|f| !f["done"].is_boolean()) {
                return Some(
                    "error: every entry on a checklist needs `done` true or false — an item whose state is unsaid is the                      one a person assumes passed."
                        .into(),
                );
            }
            // A label with no value says nothing; a value with no label is a number
            // with no meaning, which is the failure a card exists to avoid.
            if facts
                .iter()
                .any(|f| f["label"].as_str().unwrap_or("").trim().is_empty() || f["value"].is_null())
            {
                return Some("error: every fact needs a non-empty `label` and a `value`".into());
            }
            if facts.len() > 8 {
                return Some(format!(
                    "error: {} facts is a table, not a card — use `table`, or pick the few a person would repeat",
                    facts.len()
                ));
            }
            return Some(format!(
                "shown in the conversation ({} fact(s)). Don't repeat the figures in prose; say what they mean.",
                facts.len()
            ));
        }
        _ => unreachable!("routed here only for its own tools"),
    };
    Some(out)
}
