use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct Extracted {
    pub name: String,
    pub mime: String,
    /// "text" (extracted), "image" (vision model), "pdf" (scanned → cloud PDF),
    /// or "unsupported".
    pub kind: String,
    pub text: Option<String>,
    pub image_base64: Option<String>,
    /// Raw PDF bytes (base64) for a scanned PDF, to hand to Claude's native PDF.
    pub pdf_base64: Option<String>,
    /// Human-readable caveat (e.g. scanned PDF, unsupported type).
    pub note: Option<String>,
}

pub(super) fn blank(name: String, mime: &str, kind: &str, note: Option<String>) -> Extracted {
    Extracted { name, mime: mime.into(), kind: kind.into(), text: None, image_base64: None, pdf_base64: None, note }
}

pub(super) const MAX_TEXT: usize = 200_000; // keep prompts sane

pub(super) const MAX_IMAGE_BYTES: usize = 12_000_000;

pub(super) fn ext_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

pub(super) fn image_mime(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

pub(super) fn is_texty(ext: &str) -> bool {
    matches!(
        ext,
        "txt" | "md" | "markdown" | "json" | "csv" | "tsv" | "log" | "yaml" | "yml" | "toml" | "ini"
            | "xml" | "html" | "htm" | "css" | "js" | "jsx" | "ts" | "tsx" | "py" | "rs" | "go"
            | "java" | "c" | "h" | "cpp" | "hpp" | "cc" | "rb" | "php" | "sh" | "bash" | "zsh"
            | "sql" | "swift" | "kt" | "scala" | "lua" | "r" | "env" | "conf" | "gitignore" | ""
    )
}

/// Read and process a file at `path`.
pub fn extract(path: &str) -> Result<Extracted, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("File not found.".into());
    }
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
    let ext = ext_of(p);

    // Images → base64 for a vision model.
    if let Some(mime) = image_mime(&ext) {
        let bytes = std::fs::read(p).map_err(|e| format!("Couldn't read image: {e}"))?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err("Image is too large (over 12 MB).".into());
        }
        let mut e = blank(name, mime, "image", None);
        e.image_base64 = Some(STANDARD.encode(&bytes));
        return Ok(e);
    }

    // PDF → extract text locally; if it's scanned (no text) keep the raw bytes so
    // the cloud (Claude) can read it natively.
    if ext == "pdf" {
        let text = pdf_extract::extract_text(p).unwrap_or_default();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            let bytes = std::fs::read(p).map_err(|e| format!("Couldn't read PDF: {e}"))?;
            let mut e = blank(name, "application/pdf", "pdf", Some("Scanned PDF (no selectable text) — switch to a Claude model to read it; local OCR isn't bundled.".into()));
            e.pdf_base64 = Some(STANDARD.encode(&bytes));
            return Ok(e);
        }
        let mut e = blank(name, "application/pdf", "text", None);
        e.text = Some(trimmed.chars().take(MAX_TEXT).collect());
        return Ok(e);
    }

    // Office documents (zip-based OOXML / spreadsheets).
    match ext.as_str() {
        "docx" => return extract_ooxml_text(p, name, "word/document.xml", false),
        "pptx" => return extract_ooxml_text(p, name, "ppt/slides/slide", true),
        "xlsx" | "xlsm" | "xls" | "ods" => return extract_spreadsheet(p, name),
        _ => {}
    }

    // Text / code → read as UTF-8.
    let bytes = std::fs::read(p).map_err(|e| format!("Couldn't read file: {e}"))?;
    if let Ok(s) = String::from_utf8(bytes) {
        if is_texty(&ext) || !s.contains('\u{0}') {
            let mime = if ext.is_empty() { "text/plain".into() } else { format!("text/{ext}") };
            let mut e = blank(name, &mime, "text", None);
            e.text = Some(s.chars().take(MAX_TEXT).collect());
            return Ok(e);
        }
    }

    Ok(blank(
        name,
        "application/octet-stream",
        "unsupported",
        Some(format!("“{}” files can't be read inline yet — try a PDF, image, Office doc, or text/code file.", if ext.is_empty() { "binary".into() } else { ext })),
    ))
}

/// Strip XML tags from an OOXML part, keeping the text nodes. Good enough for
/// pulling the readable text out of Word/PowerPoint document parts.
pub(super) fn strip_xml_text(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut last_tag = String::new();
    for ch in xml.chars() {
        match ch {
            '<' => { in_tag = true; last_tag.clear(); }
            '>' => {
                in_tag = false;
                // paragraph / break tags become whitespace so words don't run together
                if last_tag.starts_with("w:p") || last_tag.starts_with("a:p") || last_tag.contains("br") || last_tag.starts_with("/w:p") {
                    out.push('\n');
                }
            }
            _ if in_tag => last_tag.push(ch),
            _ => out.push(ch),
        }
    }
    // unescape the few entities OOXML uses, collapse blank runs
    let out = out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
    out.lines().map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n")
}

pub(super) fn extract_ooxml_text(p: &Path, name: String, prefix: &str, multi: bool) -> Result<Extracted, String> {
    let file = std::fs::File::open(p).map_err(|e| format!("Couldn't open document: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Not a valid Office file: {e}"))?;
    // collect matching parts (one for docx, many slides for pptx)
    let mut parts: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| if multi { n.starts_with(prefix) && n.ends_with(".xml") } else { n == prefix })
        .collect();
    parts.sort();
    let mut text = String::new();
    for part in &parts {
        if let Ok(mut f) = zip.by_name(part) {
            let mut buf = String::new();
            use std::io::Read;
            if f.read_to_string(&mut buf).is_ok() {
                let t = strip_xml_text(&buf);
                if !t.trim().is_empty() {
                    if !text.is_empty() { text.push_str("\n\n"); }
                    text.push_str(&t);
                }
            }
        }
    }
    let mime = "application/vnd.openxmlformats";
    if text.trim().is_empty() {
        return Ok(blank(name, mime, "unsupported", Some("This document has no extractable text.".into())));
    }
    let mut e = blank(name, mime, "text", None);
    e.text = Some(text.chars().take(MAX_TEXT).collect());
    Ok(e)
}

pub(super) fn extract_spreadsheet(p: &Path, name: String) -> Result<Extracted, String> {
    use calamine::{open_workbook_auto, Reader};
    let mut wb = open_workbook_auto(p).map_err(|e| format!("Couldn't open spreadsheet: {e}"))?;
    let mut text = String::new();
    let sheets = wb.sheet_names().to_vec();
    for sheet in sheets {
        if let Ok(range) = wb.worksheet_range(&sheet) {
            if range.is_empty() { continue; }
            text.push_str(&format!("# Sheet: {sheet}\n"));
            for row in range.rows() {
                let line = row.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("\t");
                if !line.trim().is_empty() {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
            text.push('\n');
        }
    }
    if text.trim().is_empty() {
        return Ok(blank(name, "application/vnd.ms-excel", "unsupported", Some("This spreadsheet is empty.".into())));
    }
    let mut e = blank(name, "application/vnd.ms-excel", "text", None);
    e.text = Some(text.chars().take(MAX_TEXT).collect());
    Ok(e)
}
