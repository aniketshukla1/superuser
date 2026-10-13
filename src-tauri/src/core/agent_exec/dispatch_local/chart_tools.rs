use super::*;

pub(super) fn chart_tools(root: &Path, _todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();
    let rel = || args["path"].as_str().unwrap_or("").to_string();

    match name {
        // Charts draw in the CONVERSATION; a file is what you ask for on top.
        //
        // `path` used to be required, so every chart left a `.chart.json` behind — one
        // session on this machine accumulated eleven of them, none of which anyone had
        // asked to keep. The transcript never needed the file: `chartsOf` reads the spec
        // straight off the tool call, so the drawing and the saving were always separate
        // things that happened to be welded together.
        //
        // Its own arm because the group below requires a path before it does anything,
        // and this one must be allowed to have none.
        "chart" => {
            let spec = &args["spec"];
            let kind = spec["type"].as_str().unwrap_or("");
            if !["bar", "line", "area", "scatter", "pie"].contains(&kind) {
                return Some("error: chart `type` must be one of bar, line, area, scatter, pie".into());
            }
            // Rows from the file, when the spec leaves them out. This is the step where a
            // chart otherwise stops matching what it says it came from: the model reads the
            // file, retypes the figures, and one of them is wrong with nothing to catch it.
            let mut spec = spec.clone();
            let mut read_rows = None;
            if spec["data"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                let source = args["source"].as_str().unwrap_or("");
                if !crate::core::dataload::is_readable(source) {
                    return Some(
                        "error: chart `data` must be a non-empty array of rows — or name a .csv, .tsv or .json file as                          `source` and leave `data` out, and its rows will be read for you."
                            .into(),
                    );
                }
                match crate::core::dataload::read(root, source) {
                    Ok(rows) => {
                        read_rows = Some((rows.rows.len(), rows.columns.join(", ")));
                        if let Some(obj) = spec.as_object_mut() {
                            obj.insert("data".into(), serde_json::Value::Array(rows.rows));
                        }
                    }
                    Err(e) => return Some(format!("error: {e}")),
                }
            }
            let spec = &spec;
            // Stamp the source into the spec either way. The gate checked it on the way in
            // (core::gates::grounding); carrying it means the chart can answer "where did
            // this come from?" both in the conversation and, if kept, a week later.
            let mut spec = spec.clone();
            if let (Some(obj), Some(src)) = (spec.as_object_mut(), args["source"].as_str()) {
                obj.insert("source".into(), serde_json::Value::String(src.trim().to_string()));
            }
            // Said back, so the model knows what was drawn rather than what it asked for.
            let from_file = read_rows
                .map(|(n, columns)| format!(" It was drawn from {n} row(s) read out of the file — columns: {columns}."))
                .unwrap_or_default();

            let r = rel();
            if r.trim().is_empty() {
                // Drawn and not kept. The transcript renders it from this call.
                return Some(format!(
                    "drawn in the conversation.{from_file} It is not saved — pass `path` only if the user asks to keep it. \
                     Don't repeat its numbers in prose; say what it shows."
                ));
            }
            if r.split('/').any(|c| c == "..") {
                return Some("error: invalid path".into());
            }
            let full = match coder::resolve_write_path(&root_s, &r) {
                Ok(p) => p,
                Err(e) => return Some(format!("error: {e}")),
            };
            // The suffix is corrected rather than rejected — it is what the panel
            // dispatches on, and a model that wrote `sales.json` had the intent right.
            let target = if full.to_string_lossy().ends_with(".chart.json") {
                full.clone()
            } else {
                let base = full.to_string_lossy().trim_end_matches(".json").trim_end_matches('.').to_string();
                std::path::PathBuf::from(format!("{base}.chart.json"))
            };
            let shown = target.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            return Some(match serde_json::to_string_pretty(&spec) {
                Ok(j) => {
                    if let Some(dir) = target.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match std::fs::write(&target, j) {
                        Ok(()) => format!(
                            "drawn in the conversation and saved as {shown}. Don't repeat its numbers in prose; say what it shows."
                        ),
                        Err(e) => format!("error writing {shown}: {e}"),
                    }
                }
                Err(e) => format!("error: {e}"),
            });
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
