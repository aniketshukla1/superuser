use super::*;

pub(super) fn table_tools(root: &Path, _todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();
    let rel = || args["path"].as_str().unwrap_or("").to_string();

    match name {
        // A table renders in the conversation; `path` writes it as CSV.
        //
        // CSV and not the spec's own JSON, because a saved table is for taking
        // somewhere else — a spreadsheet, a script, a message — and nothing outside
        // this app reads our shape. A chart is different: its file IS how the panel
        // redraws it, so that one keeps its JSON.
        "table" => {
            let spec = &args["spec"];
            let cols = spec["columns"].as_array().cloned().unwrap_or_default();
            if cols.is_empty() {
                return Some("error: table `columns` must be a non-empty array of {field, label}".into());
            }
            if cols.iter().any(|c| c["field"].as_str().unwrap_or("").trim().is_empty()) {
                return Some("error: every column needs a non-empty `field`".into());
            }
            let mut spec = spec.clone();
            let mut rows = spec["rows"].as_array().cloned().unwrap_or_default();
            let mut from_file = String::new();
            if rows.is_empty() {
                // The same trade as a chart's: name the file and the rows are read, rather
                // than retyped into the spec by the model that read it.
                let source = args["source"].as_str().unwrap_or("");
                if !crate::core::dataload::is_readable(source) {
                    return Some(
                        "error: table `rows` must be a non-empty array — or name a .csv, .tsv or .json file as `source` \
                         and leave `rows` out, and its rows will be read for you."
                            .into(),
                    );
                }
                match crate::core::dataload::read(root, source) {
                    Ok(read) => {
                        from_file = format!(" Read from the file — columns in it: {}.", read.columns.join(", "));
                        rows = read.rows;
                        if let Some(obj) = spec.as_object_mut() {
                            obj.insert("rows".into(), serde_json::Value::Array(rows.clone()));
                        }
                    }
                    Err(e) => return Some(format!("error: {e}")),
                }
            }
            let spec = &spec;

            let r = rel();
            if r.trim().is_empty() {
                return Some(format!(
                    "shown in the conversation: {} row(s), {} column(s).{from_file} It is not saved — pass `path` only if the user asks to keep it. \
                     Don't repeat the rows in prose; say what they show.",
                    rows.len(),
                    cols.len()
                ));
            }
            if r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            let target = if full.extension().and_then(|e| e.to_str()) == Some("csv") {
                full.clone()
            } else {
                std::path::PathBuf::from(format!("{}.csv", full.to_string_lossy()))
            };
            // Quote every field rather than guessing which need it: a value with a
            // comma, a quote or a newline in it is the case that corrupts a CSV
            // silently, and the reader that opens it has no way to tell.
            let esc = |v: &serde_json::Value| -> String {
                let raw = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => String::new(),
                    other => other.to_string(),
                };
                format!("\"{}\"", raw.replace('"', "\"\""))
            };
            let mut out = cols
                .iter()
                .map(|c| esc(&serde_json::Value::String(
                    c["label"].as_str().unwrap_or_else(|| c["field"].as_str().unwrap_or("")).to_string(),
                )))
                .collect::<Vec<_>>()
                .join(",");
            for row in &rows {
                out.push('\n');
                out.push_str(
                    &cols
                        .iter()
                        .map(|c| esc(&row[c["field"].as_str().unwrap_or("")]))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            out.push('\n');
            let shown = target.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            if let Some(dir) = target.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            return Some(match std::fs::write(&target, out) {
                Ok(()) => format!(
                    "shown in the conversation and saved as {shown} ({} rows). Don't repeat the rows in prose; say what they show.",
                    rows.len()
                ),
                Err(e) => format!("error writing {shown}: {e}"),
            });
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
