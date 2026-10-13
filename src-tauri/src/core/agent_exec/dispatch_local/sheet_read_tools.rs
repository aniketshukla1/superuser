use super::*;

pub(super) fn sheet_read_tools(root: &Path, _todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();
    let rel = || args["path"].as_str().unwrap_or("").to_string();

    let out = match name {
        "sheet_read" | "sheet_new" | "sheet_write" | "sheet_sources" | "sheet_profile" => {
            let r = rel();
            if r.trim().is_empty() || r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            // Through the same checker the file tools use: a workbook is a file, and
            // a document tool must not be a way around the workspace boundary or a
            // symlink out of it.
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            let sheet = args["sheet"].as_str().filter(|s| !s.trim().is_empty());
            match name {
                "sheet_sources" => {
                    let l = crate::core::provenance::load(&full);
                    crate::core::provenance::describe(&l, 60)
                }
                "sheet_read" => match crate::core::sheets::read_rows(&full, sheet, 200, 40) {
                    Ok(rows) => {
                        let names = crate::core::sheets::sheet_names(&full).unwrap_or_default();
                        let body = rows
                            .iter()
                            .enumerate()
                            .map(|(i, r)| format!("{}: {}", i + 1, r.join(" | ")))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("sheets: {}\n{}", names.join(", "), body)
                    }
                    Err(e) => format!("error: {e}"),
                },
                // Profile, not a dump: see core/tableprofile.rs for why the shape of a table
                // is a cheaper and more useful answer than its contents.
                "sheet_profile" => {
                    // Generous row cap — a profile of the first 40 rows would report a
                    // sample, and the whole point is to describe the file.
                    match crate::core::sheets::read_rows(&full, sheet, 5000, 200) {
                        Ok(rows) => {
                            let p = crate::core::tableprofile::profile(&rows);
                            match serde_json::to_string_pretty(&p) {
                                Ok(j) => format!(
                                    "sheets: {}\n{j}",
                                    crate::core::sheets::sheet_names(&full).unwrap_or_default().join(", ")
                                ),
                                Err(e) => format!("error: {e}"),
                            }
                        }
                        Err(e) => format!("error: {e}"),
                    }
                }
                // A chart is a FILE, so it survives the turn, reopens, and can be handed to
                // something else. Validated here rather than trusted: a spec the panel cannot
                // draw should fail now, with a message the model can act on, not render as an
                // empty box later.
                "sheet_new" => {
                    let sheets: Vec<String> = args["sheets"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    match crate::core::sheets::create(&full, &sheets) {
                        Ok(()) => format!("created {r} with sheet(s): {}", if sheets.is_empty() { "Sheet1".into() } else { sheets.join(", ") }),
                        Err(e) => format!("error: {e}"),
                    }
                }
                _ => {
                    let edits: Vec<crate::core::sheets::CellEdit> = args["cells"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| {
                                    Some(crate::core::sheets::CellEdit {
                                        cell: v["cell"].as_str()?.to_string(),
                                        value: match &v["value"] {
                                            serde_json::Value::String(s) => s.clone(),
                                            serde_json::Value::Null => String::new(),
                                            other => other.to_string(),
                                        },
                                        // The model has no formula engine, so a
                                        // formula it writes has no cached result to
                                        // record. Unchanged behaviour — see the
                                        // note on `CellEdit::result`.
                                        result: None,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if edits.is_empty() {
                        return Some("error: sheet_write needs at least one cell".into());
                    }
                    match crate::core::sheets::write_cells(&full, sheet, &edits) {
                        Ok(n) => {
                            // Record where the values came from, beside the workbook.
                            // Only for cells that actually named a source: an invented
                            // citation is worse than none, so nothing is inferred here.
                            let sheet_name = sheet.unwrap_or("Sheet1");
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs_f64())
                                .unwrap_or(0.0);
                            let sourced: Vec<(String, crate::core::provenance::Source)> = args["cells"]
                                .as_array()
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| {
                                            let cell = v["cell"].as_str()?.to_string();
                                            let source = v["source"].as_str().filter(|s| !s.trim().is_empty())?;
                                            Some((
                                                cell,
                                                crate::core::provenance::Source {
                                                    source: source.to_string(),
                                                    kind: if source.starts_with("http") { "web".into() } else { "file".into() },
                                                    quote: v["quote"].as_str().map(String::from),
                                                    at: now,
                                                },
                                            ))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            let cited = sourced.len();
                            if cited > 0 {
                                let _ = crate::core::provenance::record(&full, sheet_name, &sourced);
                            }
                            match cited {
                                0 => format!("set {n} cell(s) in {r}"),
                                c => format!("set {n} cell(s) in {r}; recorded a source for {c}"),
                            }
                        }
                        Err(e) => format!("error: {e}"),
                    }
                }
            }
        }
        "edit_file" => {
            // str_replace validates old_str matches exactly once and returns the full
            // new content (it's a proposal generator for the Coder); an assistant executes,
            // so we persist the result to disk.
            let r = rel();
            match coder::str_replace(&root_s, &r, args["old_str"].as_str().unwrap_or(""), args["new_str"].as_str().unwrap_or("")) {
                // `str_replace` already refused anything outside the workspace (it
                // reads through `within` first). Resolving again costs nothing and
                // means the write can't drift from the check that authorised it.
                Ok((_old, new_content)) => match coder::resolve_write_path(&root_s, &r).and_then(|p| std::fs::write(p, &new_content).map_err(|e| e.to_string())) {
                    Ok(_) => format!("edited {r}"),
                    Err(e) => format!("error writing {r}: {e}"),
                },
                Err(e) => format!("error: {e}"),
            }
        }
        "list_dir" => {
            let dir = root.join(rel());
            match std::fs::read_dir(&dir) {
                Ok(rd) => {
                    let mut names: Vec<String> = rd
                        .flatten()
                        .map(|e| {
                            let n = e.file_name().to_string_lossy().into_owned();
                            if e.path().is_dir() { format!("{n}/") } else { n }
                        })
                        .collect();
                    names.sort();
                    if names.is_empty() { "(empty)".into() } else { names.join("\n") }
                }
                Err(e) => format!("error: {e}"),
            }
        }
        "grep" => coder::grep(&root_s, args["pattern"].as_str().unwrap_or(""), 12000),
        _ => unreachable!("routed here only for its own tools"),
    };
    Some(out)
}
