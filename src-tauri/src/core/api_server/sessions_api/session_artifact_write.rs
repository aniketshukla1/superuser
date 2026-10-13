use super::super::*;

/// The listing's `kind` — what icon and renderer the panel should reach for.
/// Mirrors `kindFromPath` in the GUI's `api.ts`; the viewer falls back to that when
/// it has no row, and the two disagreeing is how a file gets one icon in the list
/// and a different renderer when opened.
pub(in crate::core::api_server) fn artifact_kind(rel: &str) -> &'static str {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    // `.chart.json` before the generic json arm: a chart is still valid JSON, and anything
    // that only looks at the last extension would file it as code.
    if rel.to_lowercase().ends_with(".chart.json") {
        return "chart";
    }
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "image",
        "html" | "htm" => "html",
        "md" => "markdown",
        "csv" => "csv",
        "pdf" => "pdf",
        "xlsx" | "xlsm" | "xls" | "ods" => "sheet",
        // These must match the `kind` the read endpoint returns for the same file,
        // or the list and the open document disagree about what it is — the row says
        // "Office document" while the viewer is showing a presentation.
        "pptx" => "slides",
        "docx" => "doc",
        "doc" | "odt" | "ppt" | "odp" => "office",
        "py" | "js" | "ts" | "tsx" | "jsx" | "json" | "sh" | "css" | "rs" | "toml" | "yaml" | "yml" => "code",
        _ => "text",
    }
}

pub(in crate::core::api_server) async fn session_artifacts(AxState(ctx): AxState<Arc<ApiContext>>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    use tauri::Manager;
    // The agent writes into agent_exec::session_dir — list THAT, not a separate
    // "work" subfolder, or the panel is always empty.
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    let items: Vec<Value> = crate::core::agent_exec::list_artifacts(&dir)
        .into_iter()
        // Neither the session record nor the run's own journal is a deliverable.
        .filter(|rel| rel != "session.json" && rel != crate::core::runlog::FILE)
        .map(|rel| {
            // `ArtifactInfo` on the client declares name/kind/modified_at as
            // present, and this endpoint used to send only path+size — so every
            // consumer rendered a blank title, a default icon and an empty
            // timestamp, with nothing to catch it: the types said the fields were
            // there and the JSON simply lacked them. Found 2026-08-21 when the
            // Assistant tab's "Recent files" came out as a list of empty rows.
            let full = dir.join(&rel);
            let md = std::fs::metadata(&full).ok();
            let modified_at = md
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            json!({
                "path": rel,
                "name": std::path::Path::new(&rel)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| rel.clone()),
                "abs_path": full.to_string_lossy(),
                "kind": artifact_kind(&rel),
                "size": md.as_ref().map(|m| m.len()).unwrap_or(0),
                "modified_at": modified_at,
            })
        })
        .collect();
    Json(json!({ "artifacts": items }))
}

/// `POST /v1/sessions/:id/artifacts/write` — save text back to a file in the
/// session's folder. Body: `{ "path": "...", "content": "..." }`.
///
/// This is what makes the panel a workspace rather than a display: until now the
/// agent could write files and the user could only look at them. Text and markdown
/// only — the same reasoning as `write_file`'s refusal in `agent_exec`: bytes
/// written under a `.xlsx`/`.docx`/`.pptx` name produce a file no application can
/// open, and doing it from an editor pane would be no less broken than doing it
/// from a tool.
///
/// Traversal is checked by the same `resolve_write_path` the file tools and the
/// read endpoint use, so a path from the browser cannot escape the session folder.
pub(in crate::core::api_server) async fn session_artifact_write(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let rel = body["path"].as_str().unwrap_or("").to_string();
    let content = body["content"].as_str().unwrap_or("");
    let fail = |msg: &str| Json(json!({ "ok": false, "error": msg, "path": rel.clone() }));
    if rel.trim().is_empty() {
        return fail("no path given");
    }
    // Same list as `agent_exec::document_write_refusal`, for the same reason.
    let ext = std::path::Path::new(&rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if matches!(
        ext.as_str(),
        "xlsx" | "xlsm" | "xls" | "ods" | "docx" | "doc" | "odt" | "pptx" | "ppt" | "odp" | "pdf"
    ) {
        return fail(&format!(
            ".{ext} is a binary document — editing it as text here would corrupt it. Open it in its own app."
        ));
    }
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    let full = match crate::core::coder::resolve_write_path(&dir.to_string_lossy(), &rel) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, content) {
        Ok(_) => Json(json!({ "ok": true, "path": rel, "bytes": content.len() })),
        Err(e) => fail(&format!("couldn't save: {e}")),
    }
}
