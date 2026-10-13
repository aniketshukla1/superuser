use super::*;

pub(super) const DOC: &str = "word/document.xml";

/// Read the document's paragraphs, in order — one string each, runs concatenated.
pub fn read_paragraphs(path: &Path) -> Result<Vec<String>, String> {
    let xml = part(path, DOC)?;
    Ok(paragraph_spans(&xml)
        .into_iter()
        .map(|(s, e)| text_of(&xml[s..e]))
        .collect())
}

/// Replace the text of the paragraphs named by `edits` (0-based, in document order).
///
/// Out-of-range indices are an error rather than a silent no-op: the caller is
/// editing a document it has just read, so a bad index means the two have drifted
/// apart, and writing the *other* paragraphs anyway would scatter the user's edits.
pub fn write_paragraphs(path: &Path, edits: &[(usize, String)]) -> Result<usize, String> {
    if edits.is_empty() {
        return Ok(0);
    }
    let xml = part(path, DOC)?;
    let spans = paragraph_spans(&xml);
    for (i, _) in edits {
        if *i >= spans.len() {
            return Err(format!(
                "this document has {} paragraphs; there is no paragraph {}",
                spans.len(),
                i + 1
            ));
        }
    }
    // Apply back-to-front so earlier spans keep their offsets.
    let mut out = xml.clone();
    let mut ordered: Vec<&(usize, String)> = edits.iter().collect();
    ordered.sort_by(|a, b| b.0.cmp(&a.0));
    for (i, text) in ordered {
        let (s, e) = spans[*i];
        out.replace_range(s..e, &rebuilt(&xml[s..e], text));
    }
    rewrite(path, DOC, &out)?;
    Ok(edits.len())
}

/// Byte ranges of each `<w:p …>…</w:p>` (or self-closing `<w:p/>`), in order.
pub(super) fn paragraph_spans(xml: &str) -> Vec<(usize, usize)> {
    let b = xml.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = xml[i..].find("<w:p") {
        let start = i + rel;
        // `<w:pPr>`, `<w:pStyle>` etc. also start with `<w:p` — only a tag whose name
        // ends here is a paragraph.
        let after = b.get(start + 4).copied();
        if !matches!(after, Some(b' ') | Some(b'>') | Some(b'/')) {
            i = start + 4;
            continue;
        }
        let Some(close_rel) = xml[start..].find('>') else { break };
        let open_end = start + close_rel + 1;
        if b[open_end - 2] == b'/' {
            out.push((start, open_end)); // <w:p/>
            i = open_end;
            continue;
        }
        match xml[open_end..].find("</w:p>") {
            Some(end_rel) => {
                let end = open_end + end_rel + "</w:p>".len();
                out.push((start, end));
                i = end;
            }
            None => break,
        }
    }
    out
}

/// The visible text of a paragraph: every `<w:t>` run, concatenated.
pub(super) fn text_of(p: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while let Some(rel) = p[i..].find("<w:t") {
        let open = i + rel;
        let Some(gt) = p[open..].find('>') else { break };
        let body = open + gt + 1;
        if p.as_bytes()[body - 2] == b'/' {
            i = body; // <w:t/>
            continue;
        }
        let Some(end_rel) = p[body..].find("</w:t>") else { break };
        out.push_str(&unescape(&p[body..body + end_rel]));
        i = body + end_rel;
    }
    out
}

/// A paragraph with the same properties and the first run's formatting, carrying
/// `text` instead of whatever it said before.
pub(super) fn rebuilt(p: &str, text: &str) -> String {
    let open_end = p.find('>').map(|i| i + 1).unwrap_or(0);
    let open = &p[..open_end];
    // Keep `<w:pPr>` — it holds the style, alignment and list membership. Losing it
    // turns a heading into body text.
    let ppr = between(p, "<w:pPr>", "</w:pPr>")
        .map(|s| format!("<w:pPr>{s}</w:pPr>"))
        .unwrap_or_default();
    // Keep the first run's `<w:rPr>` so an edited bold line stays bold.
    let rpr = between(p, "<w:rPr>", "</w:rPr>")
        .map(|s| format!("<w:rPr>{s}</w:rPr>"))
        .unwrap_or_else(|| "<w:rPr />".into());
    format!(
        "{open}{ppr}<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        crate::core::docwrite::esc(text)
    )
}

pub(super) fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some(&s[a..b])
}

pub(super) fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// One part of the archive, as text.
pub(crate) fn part(path: &Path, name: &str) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("couldn't open {}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("not a .docx: {e}"))?;
    let mut f = zip.by_name(name).map_err(|_| format!("{name} is missing — is this really a .docx?"))?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

/// Replace one part, copying every other entry across untouched.
///
/// Shared with `pptedit`: a .pptx is the same kind of archive with different parts
/// inside, and the rule that everything not named survives is the same rule.
pub(crate) fn rewrite(path: &Path, name: &str, body: &str) -> Result<(), String> {
    let src = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&src)).map_err(|e| e.to_string())?;
    // Written beside the original and renamed over it: a crash mid-write would
    // otherwise leave a truncated archive where the user's document used to be.
    let tmp = path.with_extension("docx.tmp");
    {
        let out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = zip::ZipWriter::new(out);
        for i in 0..zip.len() {
            let mut e = zip.by_index(i).map_err(|err| err.to_string())?;
            let opts: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(e.compression());
            w.start_file(e.name().to_string(), opts).map_err(|err| err.to_string())?;
            if e.name() == name {
                w.write_all(body.as_bytes()).map_err(|err| err.to_string())?;
            } else {
                let mut buf = Vec::new();
                e.read_to_end(&mut buf).map_err(|err| err.to_string())?;
                w.write_all(&buf).map_err(|err| err.to_string())?;
            }
        }
        w.finish().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("couldn't save: {e}"))
}
