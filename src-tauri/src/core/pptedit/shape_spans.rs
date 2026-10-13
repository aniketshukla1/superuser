use super::*;

/// The text of every shape on every slide, in document order.
///
/// Indices are what the caller edits by, so they must be stable: a shape with no text
/// still occupies its position and comes back as an empty string.
pub fn read_slides(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let names = slide_parts(path)?;
    let mut out = Vec::new();
    for name in &names {
        let xml = crate::core::docedit::part(path, name)?;
        out.push(shape_spans(&xml).into_iter().map(|(s, e)| text_of(&xml[s..e])).collect());
    }
    Ok(out)
}

/// Replace the text of `(slide, shape)` pairs. Both are 0-based, in document order.
///
/// Out-of-range indices are refused rather than skipped: the caller is editing a deck
/// it has just read, so a bad index means its view and the file have drifted apart,
/// and applying the rest would scatter the edits across the wrong shapes.
pub fn write_slides(path: &Path, edits: &[(usize, usize, String)]) -> Result<usize, String> {
    if edits.is_empty() {
        return Ok(0);
    }
    let names = slide_parts(path)?;

    // Grouped by slide: each slide is one part, rewritten once however many of its
    // shapes changed.
    let mut by_slide: std::collections::BTreeMap<usize, Vec<(usize, &str)>> = Default::default();
    for (slide, shape, text) in edits {
        if *slide >= names.len() {
            return Err(format!("this deck has {} slides; there is no slide {}", names.len(), slide + 1));
        }
        by_slide.entry(*slide).or_default().push((*shape, text.as_str()));
    }

    // Every index is checked before anything is written, so a bad one leaves the file
    // untouched rather than half-edited.
    let mut planned: Vec<(String, String)> = Vec::new();
    for (slide, shapes) in &by_slide {
        let name = &names[*slide];
        let xml = crate::core::docedit::part(path, name)?;
        let spans = shape_spans(&xml);
        for (shape, _) in shapes {
            if *shape >= spans.len() {
                return Err(format!(
                    "slide {} has {} shapes; there is no shape {}",
                    slide + 1,
                    spans.len(),
                    shape + 1
                ));
            }
        }
        // Back-to-front so earlier spans keep their offsets.
        let mut out = xml.clone();
        let mut ordered = shapes.clone();
        ordered.sort_by(|a, b| b.0.cmp(&a.0));
        for (shape, text) in ordered {
            let (s, e) = spans[shape];
            out.replace_range(s..e, &rebuilt(&xml[s..e], text));
        }
        planned.push((name.clone(), out));
    }
    for (name, body) in &planned {
        crate::core::docedit::rewrite(path, name, body)?;
    }
    Ok(edits.len())
}

/// `ppt/slides/slideN.xml`, ordered by N — `slide10` must not sort before `slide2`.
pub(super) fn slide_parts(path: &Path) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("couldn't open {}: {e}", path.display()))?;
    let zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).map_err(|e| format!("not a .pptx: {e}"))?;
    let mut names: Vec<String> = zip
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(String::from)
        .collect();
    names.sort_by_key(|n| {
        n.trim_start_matches("ppt/slides/slide").trim_end_matches(".xml").parse::<u32>().unwrap_or(0)
    });
    if names.is_empty() {
        return Err("this .pptx has no slides".into());
    }
    Ok(names)
}

/// Byte ranges of each `<p:sp …>…</p:sp>`, counting nesting — a group shape contains
/// shapes, and taking the first closing tag would end the outer one early.
pub(super) fn shape_spans(xml: &str) -> Vec<(usize, usize)> {
    const OPEN: &str = "<p:sp";
    const CLOSE: &str = "</p:sp>";
    let b = xml.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = xml[i..].find(OPEN) {
        let start = i + rel;
        // `<p:spPr` and `<p:spTree` also begin with `<p:sp`.
        if !matches!(b.get(start + OPEN.len()), Some(b' ') | Some(b'>') | Some(b'/')) {
            i = start + OPEN.len();
            continue;
        }
        let Some(gt) = xml[start..].find('>') else { break };
        let after_open = start + gt + 1;
        if b[after_open - 2] == b'/' {
            i = after_open;
            continue; // <p:sp/> carries no text
        }
        let mut depth = 1usize;
        let mut j = after_open;
        let mut end = None;
        while depth > 0 {
            let next_open = xml[j..].find(OPEN).map(|r| j + r);
            let next_close = xml[j..].find(CLOSE).map(|r| j + r);
            match (next_open, next_close) {
                (Some(o), Some(c)) if o < c => {
                    if matches!(b.get(o + OPEN.len()), Some(b' ') | Some(b'>') | Some(b'/')) {
                        depth += 1;
                    }
                    j = o + OPEN.len();
                }
                (_, Some(c)) => {
                    depth -= 1;
                    j = c + CLOSE.len();
                    if depth == 0 {
                        end = Some(j);
                    }
                }
                _ => break,
            }
        }
        match end {
            Some(e) => {
                out.push((start, e));
                i = e;
            }
            None => break,
        }
    }
    out
}

/// A shape's visible text: every `<a:t>` run, with paragraphs separated by newlines.
pub(super) fn text_of(sp: &str) -> String {
    let Some(body) = slice_between(sp, "<p:txBody>", "</p:txBody>") else { return String::new() };
    let mut paras: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(rel) = body[i..].find("<a:p") {
        let s = i + rel;
        if !matches!(body.as_bytes().get(s + 4), Some(b' ') | Some(b'>') | Some(b'/')) {
            i = s + 4;
            continue;
        }
        let end = match body[s..].find("</a:p>") {
            Some(r) => s + r + "</a:p>".len(),
            None => break,
        };
        paras.push(runs_text(&body[s..end]));
        i = end;
    }
    paras.join("\n")
}

pub(super) fn runs_text(p: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while let Some(rel) = p[i..].find("<a:t") {
        let open = i + rel;
        let Some(gt) = p[open..].find('>') else { break };
        let body = open + gt + 1;
        if p.as_bytes()[body - 2] == b'/' {
            i = body;
            continue;
        }
        let Some(end_rel) = p[body..].find("</a:t>") else { break };
        out.push_str(&unescape(&p[body..body + end_rel]));
        i = body + end_rel;
    }
    out
}
