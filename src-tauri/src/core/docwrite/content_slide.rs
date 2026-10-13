use super::*;

/// One slide: a heading and its bullets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub title: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

/// XML-escape. Text from a model routinely contains `&` and quotes, and an
/// unescaped one does not corrupt the words — it makes the whole file unopenable.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // A control character is legal in a Rust string and illegal in XML.
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

/// Write a Word document: each paragraph a line, the first optionally a heading.
pub fn write_docx(path: &Path, title: Option<&str>, paragraphs: &[String]) -> Result<(), String> {
    use docx_rs::*;
    let mut doc = Docx::new();
    if let Some(t) = title.map(str::trim).filter(|t| !t.is_empty()) {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(t).bold().size(36)),
        );
    }
    for p in paragraphs {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(p)));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|e| format!("couldn't create {}: {e}", path.display()))?;
    doc.build().pack(file).map_err(|e| format!("couldn't write the document: {e}"))?;
    Ok(())
}

// Deck design.
//
// A slide is a layout, and `title + bullets` dropped into empty placeholders is not
// one — it inherits whatever the master says and lands as black Calibri on white,
// which is what this used to produce. The content the caller gives us is the same;
// what changes is that every shape now carries its own geometry and styling, so the
// deck looks composed rather than defaulted, and the panel (which reads that
// geometry back) has something real to draw.
//
// Two layouts, chosen by the content: a slide with no bullets is a title/section
// slide, everything else is a content slide.

/// 16:9 at 13.333in × 7.5in, in EMU.
pub(super) const SLIDE_W: f64 = 13.333;

pub(super) const SLIDE_H: f64 = 7.5;

pub(super) const MARGIN: f64 = 0.9;

pub(super) const INK: &str = "16202C";

pub(super) const MUTED: &str = "5C6672";

pub(super) const ACCENT: &str = "C8622A";

pub(super) const PAPER: &str = "FBF9F6";

pub(super) fn emu(inches: f64) -> i64 {
    (inches * 914_400.0).round() as i64
}

/// A rectangle with an optional solid fill and optional text.
#[allow(clippy::too_many_arguments)]
pub(super) fn box_xml(id: u32, name: &str, x: f64, y: f64, w: f64, h: f64, fill: Option<&str>, body: &str) -> String {
    let fill = fill
        .map(|c| format!("<a:solidFill><a:srgbClr val=\"{c}\"/></a:solidFill>"))
        .unwrap_or_else(|| "<a:noFill/>".into());
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{name}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{ox}" y="{oy}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{fill}</p:spPr><p:txBody><a:bodyPr wrap="square" lIns="0" tIns="0" rIns="0" bIns="0"><a:normAutofit/></a:bodyPr><a:lstStyle/>{body}</p:txBody></p:sp>"#,
        ox = emu(x), oy = emu(y), cx = emu(w), cy = emu(h),
    )
}

/// One styled paragraph. `size` is in points.
pub(super) fn para(text: &str, size: f64, bold: bool, color: &str, spacing_before: Option<i64>) -> String {
    let before = spacing_before
        .map(|p| format!("<a:spcBef><a:spcPts val=\"{p}\"/></a:spcBef>"))
        .unwrap_or_default();
    format!(
        r#"<a:p><a:pPr>{before}<a:buNone/></a:pPr><a:r><a:rPr lang="en-US" sz="{sz}" b="{b}" dirty="0"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:latin typeface="Helvetica Neue"/></a:rPr><a:t>{t}</a:t></a:r></a:p>"#,
        sz = (size * 100.0).round() as i64,
        b = if bold { 1 } else { 0 },
        t = esc(text),
    )
}

/// The XML for one slide, laid out rather than dropped into placeholders.
pub(super) fn slide_xml(s: &Slide) -> String {
    let shapes = if s.bullets.is_empty() { title_slide(s) } else { content_slide(s) };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{bg}{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#,
        bg = box_xml(2, "Background", 0.0, 0.0, SLIDE_W, SLIDE_H, Some(PAPER), "<a:p/>"),
        shapes = shapes,
    )
}

/// A slide with nothing but a title: the opener, or a section divider.
pub(super) fn title_slide(s: &Slide) -> String {
    // A rule rather than a box — the title should carry the slide, with the accent
    // only marking where it starts.
    let rule = box_xml(3, "Accent", MARGIN, 2.45, 0.055, 1.5, Some(ACCENT), "<a:p/>");
    let title = box_xml(
        4, "Title", MARGIN + 0.35, 2.35, SLIDE_W - (MARGIN * 2.0) - 0.35, 2.0, None,
        &para(&s.title, 48.0, true, INK, None),
    );
    format!("{rule}{title}")
}

/// A slide with points to make.
pub(super) fn content_slide(s: &Slide) -> String {
    let inner = SLIDE_W - MARGIN * 2.0;
    let mut out = String::new();
    // The eyebrow gives the eye somewhere to land before the title, and gives the
    // deck a rhythm when slides are seen one after another.
    out.push_str(&box_xml(
        3, "Eyebrow", MARGIN, 0.78, inner, 0.3, None,
        &para("•", 11.0, true, ACCENT, None),
    ));
    out.push_str(&box_xml(
        4, "Title", MARGIN, 1.12, inner, 1.0, None,
        &para(&s.title, 30.0, true, INK, None),
    ));
    out.push_str(&box_xml(5, "Rule", MARGIN, 2.24, inner, 0.012, Some("E2DCD3"), "<a:p/>"));

    // Bullets as a single text box: PowerPoint flows them, and one box keeps the
    // spacing even however long the lines wrap.
    let body: String = s
        .bullets
        .iter()
        .enumerate()
        .map(|(i, b)| para(b, 15.0, false, MUTED, Some(if i == 0 { 0 } else { 900 })))
        .collect();
    out.push_str(&box_xml(
        6, "Body", MARGIN, 2.62, inner, SLIDE_H - 2.62 - 0.8, None,
        if body.is_empty() { "<a:p/>" } else { &body },
    ));
    out
}
