use super::*;

/// Bytes in a named charset, as text. UTF-8 and ASCII as themselves, the Latin-1 family byte
/// for character; anything else is read as UTF-8, with what doesn't decode replaced.
pub(super) fn to_text(bytes: &[u8], charset: &str) -> String {
    match charset.trim().to_ascii_lowercase().as_str() {
        "iso-8859-1" | "latin1" | "latin-1" | "windows-1252" | "cp1252" | "iso-8859-15" => bytes.iter().map(|&b| b as char).collect(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// RFC 2047: `=?utf-8?B?...?=` and `=?utf-8?Q?...?=` back into text. The space between two
/// encoded words belongs to neither, so it is dropped.
pub fn decode_words(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut last_was_word = false;
    while let Some(start) = rest.find("=?") {
        let (before, candidate) = rest.split_at(start);
        match encoded_word(candidate) {
            Some((text, used)) => {
                if !(last_was_word && before.trim().is_empty()) {
                    out.push_str(before);
                }
                out.push_str(&text);
                rest = &candidate[used..];
                last_was_word = true;
            }
            None => {
                out.push_str(before);
                out.push_str("=?");
                rest = &candidate[2..];
                last_was_word = false;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One encoded word at the start of `s`: its text, and how many bytes it spanned.
pub(super) fn encoded_word(s: &str) -> Option<(String, usize)> {
    let inner = s.strip_prefix("=?")?;
    let (charset, rest) = inner.split_once('?')?;
    let (encoding, rest) = rest.split_once('?')?;
    let end = rest.find("?=")?;
    let payload = &rest[..end];
    if payload.contains(' ') {
        return None;
    }
    let bytes = match encoding.to_ascii_uppercase().as_str() {
        "B" => B64.decode(payload).ok()?,
        "Q" => qp_decode(payload.as_bytes(), true),
        _ => return None,
    };
    let used = 2 + charset.len() + 1 + encoding.len() + 1 + end + 2;
    Some((to_text(&bytes, charset.split('*').next().unwrap_or(charset)), used))
}

/// An HTML part's words: scripts and styles dropped, blocks turned into line breaks, the
/// common entities decoded.
pub fn strip_html(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut text = String::new();
    let mut i = 0;
    while i < html.len() {
        if lower[i..].starts_with("<script") || lower[i..].starts_with("<style") {
            let close = if lower[i..].starts_with("<script") { "</script>" } else { "</style>" };
            i = lower[i..].find(close).map(|p| i + p + close.len()).unwrap_or(html.len());
            continue;
        }
        if html[i..].starts_with('<') {
            let end = html[i..].find('>').map(|p| i + p + 1).unwrap_or(html.len());
            let tag = &lower[i..end];
            if ["<br", "</p", "</div", "</tr", "</li", "</h1", "</h2", "</h3", "<p ", "<p>"].iter().any(|t| tag.starts_with(t)) {
                text.push('\n');
            }
            i = end;
            continue;
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        text.push(ch);
        i += ch.len_utf8();
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let mut out = String::new();
    let mut blank = 0;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

/// The address in a `From`-style header, lowercased: `Ada <ADA@x.com>` → `ada@x.com`.
pub fn address(value: &str) -> String {
    let inner = match (value.rfind('<'), value.rfind('>')) {
        (Some(a), Some(b)) if a < b => &value[a + 1..b],
        _ => value.split_whitespace().find(|w| w.contains('@')).unwrap_or(""),
    };
    inner.trim().trim_matches(|c| c == '"' || c == '\'' || c == ',').to_ascii_lowercase()
}

/// What the sender wrote this time: quoted history below an "On … wrote:" line, an
/// "Original Message" block, or `>` lines is cut, and the rest capped at `TEXT_CAP`.
pub fn strip_quoted(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("-----Original Message-----") || t.starts_with("________________________________") {
            break;
        }
        if t.starts_with("On ") && t.ends_with("wrote:") {
            break;
        }
        if t.starts_with('>') {
            continue;
        }
        out.push(line.trim_end());
    }
    let joined = out.join("\n");
    let trimmed = joined.trim();
    match trimmed.char_indices().nth(TEXT_CAP) {
        Some((at, _)) => format!("{}\n\n[…the rest of this email was cut]", &trimmed[..at]),
        None => trimmed.to_string(),
    }
}
