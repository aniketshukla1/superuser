use super::super::*;

/// `POST /v1/sessions/:id/artifacts/upload` — put a dropped file into the session's folder.
/// Body: `{ "path": "sales.xlsx", "data_url": "data:...;base64,..." }`
///
/// Separate from `artifacts/write` because that one is the text editor's save: it takes a
/// string and REFUSES binary documents, correctly — writing a .xlsx as text corrupts it.
/// A dropped file is the opposite case: bytes that must land unaltered, and a spreadsheet is
/// the whole point. So this decodes and writes bytes, and refuses nothing on the grounds of
/// extension.
///
/// It goes through the same `resolve_write_path` as everything else, so the session folder
/// is still the boundary and a `..` or a symlink still cannot leave it.
pub(in crate::core::api_server) async fn session_artifact_upload(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use tauri::Manager;

    // 25MB. A cap belongs here rather than only in the GUI: this is an HTTP endpoint on a
    // port anything local can reach, and "the UI would never send that" is not a limit.
    const MAX: usize = 25 * 1024 * 1024;

    let rel = body["path"].as_str().unwrap_or("").to_string();
    let fail = |msg: &str| Json(json!({ "ok": false, "error": msg, "path": rel.clone() }));
    if rel.trim().is_empty() {
        return fail("no path given");
    }
    let raw = body["data_url"].as_str().unwrap_or("");
    // Accept a data: URL or bare base64 — the browser produces the former, and a caller
    // holding just the payload should not have to fake a prefix.
    let b64 = raw.rsplit_once("base64,").map(|(_, b)| b).unwrap_or(raw);
    let bytes = match STANDARD.decode(b64.trim()) {
        Ok(b) => b,
        Err(e) => return fail(&format!("couldn't decode the file: {e}")),
    };
    if bytes.len() > MAX {
        return fail(&format!(
            "{} MB is over the {} MB upload limit",
            bytes.len() / 1024 / 1024,
            MAX / 1024 / 1024
        ));
    }

    // Where the AGENT will look, which is not always the session's own folder: a session
    // bound to a project works in that project, so a file dropped into it must land there or
    // the tools cannot see it. Found the hard way — the upload succeeded, the turn started,
    // and sheet_profile reported it could not open a file that was sitting on disk one
    // directory away.
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let ws = store(&ctx).get(&id).map(|s| s.workspace).unwrap_or_default();
    let dir = if ws.trim().is_empty() {
        crate::core::agent_exec::session_dir_path(&data_dir, &id)
    } else {
        std::path::PathBuf::from(&ws)
    };
    // Created BEFORE resolving: resolve_write_path canonicalises the root, so a session that
    // has never run has no folder and every upload into it failed with "bad workspace". That
    // is precisely the case this endpoint exists for — dropping a file into a new chat, as
    // the first thing you do.
    let _ = std::fs::create_dir_all(&dir);
    let full = match crate::core::coder::resolve_write_path(&dir.to_string_lossy(), &rel) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, &bytes) {
        Ok(_) => Json(json!({ "ok": true, "path": rel, "bytes": bytes.len() })),
        Err(e) => fail(&format!("couldn't save: {e}")),
    }
}

/// `POST /v1/sessions/:id/artifacts/sheet` — edit cells in a workbook.
/// Body: `{ "path": "...", "sheet": "Subscriptions", "edits": [{"cell":"B2","value":"12.99"}] }`
///
/// The panel's grid is an editor, not a picture of one, and this is the write half.
/// It goes through `sheets::write_cells` rather than rewriting the file, which means
/// the parts of the workbook nobody touched survive, and each value goes through
/// `interpret` — so `007` stays text, `=SUM(B2:B9)` becomes a formula, and a number
/// becomes a number. Typing into a cell here does the same thing as the agent's
/// `sheet_write` tool because it *is* the same code.
pub(in crate::core::api_server) async fn session_sheet_write(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let rel = body["path"].as_str().unwrap_or("").to_string();
    let fail = |msg: &str| Json(json!({ "ok": false, "error": msg, "path": rel.clone() }));
    if rel.trim().is_empty() {
        return fail("no path given");
    }
    let ext = std::path::Path::new(&rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "xlsx" | "xlsm") {
        return fail("only .xlsx/.xlsm can be edited here");
    }
    let edits: Vec<crate::core::sheets::CellEdit> = body["edits"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    Some(crate::core::sheets::CellEdit {
                        cell: e["cell"].as_str()?.to_string(),
                        value: e["value"].as_str().unwrap_or("").to_string(),
                        // The panel has a formula engine (Univer) and knows what the
                        // formula evaluates to; passing it through is what keeps the
                        // cell readable by everything that doesn't recalculate.
                        result: e["result"].as_str().map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    if edits.is_empty() {
        return fail("no edits given");
    }
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    let full = match crate::core::coder::resolve_write_path(&dir.to_string_lossy(), &rel) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if !full.exists() {
        return fail("that workbook isn't here any more");
    }
    let sheet = body["sheet"].as_str().filter(|s| !s.trim().is_empty());
    match crate::core::sheets::write_cells(&full, sheet, &edits) {
        Ok(n) => Json(json!({ "ok": true, "path": rel, "written": n })),
        Err(e) => fail(&e),
    }
}

/// `POST /v1/sessions/:id/artifacts/doc` — edit paragraphs of a .docx.
/// Body: `{ "path": "...", "edits": [{ "index": 2, "text": "..." }] }`
///
/// Goes through `docedit`, which rewrites only the paragraphs named and copies the
/// rest of the archive across untouched. Regenerating the file with `write_docx`
/// instead would be much simpler and would silently flatten every heading, table and
/// image in a document the user did not expect to lose.
pub(in crate::core::api_server) async fn session_doc_write(
    AxState(ctx): AxState<Arc<ApiContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use tauri::Manager;
    let rel = body["path"].as_str().unwrap_or("").to_string();
    let fail = |msg: &str| Json(json!({ "ok": false, "error": msg, "path": rel.clone() }));
    if rel.trim().is_empty() {
        return fail("no path given");
    }
    if !rel.to_lowercase().ends_with(".docx") {
        return fail("only .docx can be edited here");
    }
    let edits: Vec<(usize, String)> = body["edits"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| Some((e["index"].as_u64()? as usize, e["text"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    if edits.is_empty() {
        return fail("no edits given");
    }
    let data_dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    let dir = crate::core::agent_exec::session_dir_path(&data_dir, &id);
    let full = match crate::core::coder::resolve_write_path(&dir.to_string_lossy(), &rel) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if !full.exists() {
        return fail("that document isn't here any more");
    }
    match crate::core::docedit::write_paragraphs(&full, &edits) {
        Ok(n) => Json(json!({ "ok": true, "path": rel, "written": n })),
        Err(e) => fail(&e),
    }
}
