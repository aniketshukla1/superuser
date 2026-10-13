use super::super::*;

/// `GET /v1/sessions/:id/artifacts/read?path=` — one artifact's content, for the
/// panel's document tabs.
///
/// The panel has always asked for this and it has never existed, so every document
/// tab opened onto nothing — for every format, not only the ones whose viewer says
/// they cannot be previewed. `readArtifact` goes through the client's
/// never-rejecting JSON helper, so the 404 arrived as `{}` and rendered as a blank
/// pane rather than as an error anybody could see.
///
/// Documents come back as TEXT, through the same extractor the agent and the
/// attachment path use: a .docx is a zip of XML, and handing that to a viewer is
/// how "this file can't be previewed here" gets written in the first place.
pub(in crate::core::api_server) async fn session_artifact_read(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    let rel = q.get("path").cloned().unwrap_or_default();
    let fail = |msg: &str| {
        Json(json!({ "ok": false, "error": msg, "path": rel.clone(), "kind": "text" }))
    };
    if rel.trim().is_empty() {
        return fail("no path given");
    }
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    // Through the same checker the file tools use, so a path from the browser
    // cannot walk out of the session's folder.
    let full = match crate::core::coder::resolve_write_path(&dir.to_string_lossy(), &rel) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if !full.exists() {
        return fail("that file isn't here any more");
    }
    if full.is_dir() {
        let entries: Vec<Value> = std::fs::read_dir(&full)
            .map(|rd| {
                rd.flatten()
                    .map(|e| {
                        let md = e.metadata().ok();
                        json!({
                            "name": e.file_name().to_string_lossy(),
                            "dir": md.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                            "size": md.as_ref().map(|m| m.len()).unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Json(json!({ "ok": true, "path": rel, "kind": "folder", "entries": entries }));
    }

    let ext = full.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    // A .docx comes back as its paragraphs, so the panel can edit them in place. Not
    // as one blob of text: the panel writes back per paragraph, which is what lets a
    // real Word document keep the styles, tables and images we cannot reproduce.
    if ext == "docx" {
        // Both halves: the file itself, for `docx-preview` to render as Word would
        // show it, and the paragraphs, for editing them one at a time. A paragraph
        // list is a poor *view* of a document — it has none of the styling, tables or
        // images — but it is the only safe thing to write back, because we cannot
        // reproduce the rest. So the view is faithful and the edit is surgical.
        let data_url = std::fs::read(&full).ok().map(|b| {
            use base64::Engine as _;
            format!(
                "data:application/vnd.openxmlformats-officedocument.wordprocessingml.document;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&b)
            )
        });
        return match crate::core::docedit::read_paragraphs(&full) {
            Ok(paragraphs) => Json(json!({
                "ok": true, "path": rel, "kind": "doc",
                "paragraphs": paragraphs, "data_url": data_url,
            })),
            // A .docx we can't parse still has readable text via the extractor; say so
            // rather than showing nothing.
            Err(e) => Json(json!({
                "ok": true, "path": rel, "kind": "office", "readonly_reason": e,
                "content": crate::core::files::extract(&full.to_string_lossy())
                    .ok().and_then(|x| x.text).unwrap_or_default(),
            })),
        };
    }
    // A .pptx is handed over as the file itself, for `pptx-preview` to draw in the
    // panel. It resolves what a hand-written renderer cannot: real decks store almost
    // no explicit geometry — measured on one, zero `<a:off>` across ten slides — and
    // inherit position, size, colour and font from their layout, master and theme.
    // Reading the shapes ourselves meant guessing every box.
    // Images and PDFs are shown, not read: the viewer has an <img> and pdf.js, and
    // turning them into text here would throw away what the user wants to look at.
    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf" | "xlsx" | "xlsm" | "xls" | "pptx") {
        let bytes = match std::fs::read(&full) {
            Ok(b) => b,
            Err(e) => return fail(&format!("couldn't read it: {e}")),
        };
        let mime = match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            _ => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        };
        let kind = match ext.as_str() {
            "pdf" => "pdf",
            "xlsx" | "xlsm" | "xls" => "sheet",
            "pptx" => "slides",
            _ => "image",
        };
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Json(json!({
            "ok": true, "path": rel, "kind": kind,
            "data_url": format!("data:{mime};base64,{b64}")
        }));
    }

    // Word and PowerPoint have no inline renderer, so they are shown as their words
    // — which is what someone opening one in a side panel wants to read anyway.
    if matches!(ext.as_str(), "docx" | "pptx" | "odt" | "ods") {
        return match crate::core::files::extract(&full.to_string_lossy()) {
            Ok(e) => {
                let text = e.text.unwrap_or_default();
                if text.trim().is_empty() {
                    Json(json!({ "ok": true, "path": rel, "kind": "text",
                        "content": e.note.unwrap_or_else(|| "(no text in this document)".into()) }))
                } else {
                    Json(json!({ "ok": true, "path": rel, "kind": "text", "content": text }))
                }
            }
            Err(err) => fail(&err),
        };
    }

    // Everything else is text. Capped: a viewer is a window, and a hundred-megabyte
    // log should not be sent through a socket to fill it.
    const MAX: usize = 400_000;
    match std::fs::read(&full) {
        Err(e) => fail(&format!("couldn't read it: {e}")),
        Ok(bytes) => {
            let truncated = bytes.len() > MAX;
            let slice = if truncated { &bytes[..MAX] } else { &bytes[..] };
            let content = String::from_utf8_lossy(slice).to_string();
            let kind = if rel.to_lowercase().ends_with(".chart.json") {
                "chart"
            } else {
                match ext.as_str() {
                    "md" => "markdown",
                    "html" | "htm" => "html",
                    "csv" => "csv",
                    _ => "text",
                }
            };
            Json(json!({ "ok": true, "path": rel, "kind": kind, "content": content, "truncated": truncated }))
        }
    }
}

/// `GET /v1/sessions/:id/run?offset=&limit=` — what the last run of this session
/// actually did, still readable after it finished.
///
/// Paged by offset so a caller can poll a running turn and resume exactly where it
/// left off, and so a long run doesn't have to come back in one piece.
pub(in crate::core::api_server) async fn session_run(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    use tauri::Manager;
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    let offset = q.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);
    let limit = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(200usize).min(1000);
    let page = crate::core::runlog::read(&dir, offset, limit);
    Json(json!({
        "events": page.events,
        "next_offset": page.next_offset,
        "total": crate::core::runlog::len(&dir),
        "truncated": page.truncated,
        // So a reader can tell "nothing more yet" from "it's finished".
        "running": is_running(&id),
    }))
}
