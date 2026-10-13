
/// The same shape, carrying `text` instead of whatever it said before.
///
/// Everything outside `<p:txBody>` — the shape's position, size, fill, outline and
/// its place in the tree — is kept byte-for-byte. Inside it, the body's own
/// properties and the first run's formatting are kept so an edited title stays a
/// title. Newlines in `text` become separate paragraphs, which is what they look
/// like on a slide.
pub(super) fn rebuilt(sp: &str, text: &str) -> String {
    let Some((body_start, body_end)) = span_between(sp, "<p:txBody>", "</p:txBody>") else {
        return sp.to_string(); // no text to replace
    };
    let body = &sp[body_start..body_end];
    let body_pr = slice_between(body, "<a:bodyPr", "/>")
        .map(|inner| format!("<a:bodyPr{inner}/>"))
        .unwrap_or_else(|| "<a:bodyPr/>".into());
    let ppr = slice_between(body, "<a:pPr", ">")
        .filter(|p| !p.contains('<'))
        .map(|p| format!("<a:pPr{p}>"))
        .unwrap_or_default();
    let rpr = slice_between(body, "<a:rPr", ">")
        .filter(|p| !p.contains('<'))
        .map(|p| format!("<a:rPr{p}>"))
        .unwrap_or_else(|| "<a:rPr lang=\"en-US\"/>".into());
    let ppr_close = if ppr.is_empty() { "" } else { "</a:pPr>" };
    let rpr_close = if rpr.ends_with("/>") { "" } else { "</a:rPr>" };
    let paras: String = text
        .split('\n')
        .map(|line| {
            format!(
                "<a:p>{ppr}{ppr_close}<a:r>{rpr}{rpr_close}<a:t>{}</a:t></a:r></a:p>",
                crate::core::docwrite::esc(line)
            )
        })
        .collect();
    format!("{}<p:txBody>{body_pr}<a:lstStyle/>{paras}</p:txBody>{}", &sp[..body_start], &sp[body_end..])
}

/// Byte range of `open …  close`, inclusive of both.
pub(super) fn span_between(s: &str, open: &str, close: &str) -> Option<(usize, usize)> {
    let a = s.find(open)?;
    let b = s[a..].find(close)? + a + close.len();
    Some((a, b))
}

/// What sits between `open` and `close`, exclusive.
pub(super) fn slice_between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some(&s[a..b])
}

pub(super) fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}
