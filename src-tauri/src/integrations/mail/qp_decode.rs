use super::*;

/// The most text a message contributes, in characters. A draft needs the question, not the
/// fifty-message thread quoted beneath it.
pub const TEXT_CAP: usize = 8_000;

/// A parsed message.
#[derive(Debug, Clone, Default)]
pub struct Parsed {
    /// Header names lowercased, values unfolded and still encoded.
    pub(super) headers: Vec<(String, String)>,
    /// What the sender wrote: the plain-text part, or the HTML part's words when there is
    /// no plain one. Quoted history is left in; `strip_quoted` takes it out.
    pub text: String,
}

impl Parsed {
    /// The first header of this name, as sent (still encoded).
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers.iter().find(|(n, _)| *n == name).map(|(_, v)| v.as_str())
    }

    /// A header decoded for a person to read: encoded words turned back into text.
    pub fn decoded(&self, name: &str) -> String {
        decode_words(self.header(name).unwrap_or(""))
    }

    pub fn has(&self, name: &str) -> bool {
        self.header(name).is_some()
    }
}

pub fn parse(raw: &[u8]) -> Parsed {
    let (head, body) = split_head(raw);
    let headers = unfold(&head);
    let mut leaves = Vec::new();
    collect_text(&headers, body, 0, &mut leaves);
    // Plain text first, as the sender meant it; HTML only when that is all there is.
    let text = leaves
        .iter()
        .find(|(mime, t)| mime == "text/plain" && !t.trim().is_empty())
        .or_else(|| leaves.iter().find(|(_, t)| !t.trim().is_empty()))
        .map(|(mime, t)| if mime == "text/html" { strip_html(t) } else { t.clone() })
        .unwrap_or_default();
    Parsed { headers, text }
}

/// Split a message at the blank line between its head and body.
pub(super) fn split_head(raw: &[u8]) -> (String, &[u8]) {
    let find = |needle: &[u8]| raw.windows(needle.len()).position(|w| w == needle);
    let (at, gap) = match (find(b"\r\n\r\n"), find(b"\n\n")) {
        (Some(a), Some(b)) if b < a => (b, 2),
        (Some(a), _) => (a, 4),
        (None, Some(b)) => (b, 2),
        (None, None) => (raw.len(), 0),
    };
    let head = String::from_utf8_lossy(&raw[..at]).into_owned();
    let body = if at + gap <= raw.len() { &raw[at + gap..] } else { &[][..] };
    (head, body)
}

/// Header lines, with a folded line (one starting with a space or tab) joined to the one
/// before it.
pub(super) fn unfold(head: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in head.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with([' ', '\t']) {
            if let Some((_, v)) = out.last_mut() {
                v.push(' ');
                v.push_str(line.trim());
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            out.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    out
}

pub(super) fn header<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()).unwrap_or("")
}

/// `text/plain; charset="utf-8"` → (`text/plain`, value of `param`).
pub(super) fn content_type(value: &str) -> (String, Vec<(String, String)>) {
    let mut parts = value.split(';');
    let mime = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let params = parts
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().trim_matches('"').to_string()))
        .collect();
    (if mime.is_empty() { "text/plain".into() } else { mime }, params)
}

pub(super) fn param<'a>(params: &'a [(String, String)], name: &str) -> Option<&'a str> {
    params.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// Every text part, decoded, in the order it appears. Attachments are skipped, and nesting
/// stops at a depth no real message reaches.
pub(super) fn collect_text(headers: &[(String, String)], body: &[u8], depth: usize, out: &mut Vec<(String, String)>) {
    if depth > 6 {
        return;
    }
    let (mime, params) = content_type(header(headers, "content-type"));
    if header(headers, "content-disposition").to_ascii_lowercase().starts_with("attachment") {
        return;
    }
    if mime.starts_with("multipart/") {
        let Some(boundary) = param(&params, "boundary") else { return };
        for part in split_parts(body, boundary) {
            let (head, part_body) = split_head(part);
            collect_text(&unfold(&head), part_body, depth + 1, out);
        }
        return;
    }
    if mime == "text/plain" || mime == "text/html" {
        let bytes = decode_transfer(body, header(headers, "content-transfer-encoding"));
        out.push((mime, to_text(&bytes, param(&params, "charset").unwrap_or(""))));
    }
}

/// The parts between `--boundary` lines, the closing `--boundary--` ending the list.
pub(super) fn split_parts<'a>(body: &'a [u8], boundary: &str) -> Vec<&'a [u8]> {
    let marker = format!("--{boundary}");
    let text = body;
    let mut starts = Vec::new();
    let mut i = 0;
    while let Some(pos) = text[i..].windows(marker.len()).position(|w| w == marker.as_bytes()) {
        let at = i + pos;
        // A boundary counts only at the start of a line.
        if at == 0 || text[at - 1] == b'\n' {
            starts.push(at);
        }
        i = at + marker.len();
    }
    let mut parts = Vec::new();
    for pair in starts.windows(2) {
        let (a, b) = (pair[0] + marker.len(), pair[1]);
        let part = &text[a..b];
        // Drop the rest of the boundary line, and the line break that belongs to the next one.
        let part = match part.iter().position(|&c| c == b'\n') {
            Some(nl) => &part[nl + 1..],
            None => &part[part.len()..],
        };
        let part = part.strip_suffix(b"\r\n").or_else(|| part.strip_suffix(b"\n")).unwrap_or(part);
        parts.push(part);
    }
    parts
}

pub(super) fn decode_transfer(body: &[u8], encoding: &str) -> Vec<u8> {
    match encoding.trim().to_ascii_lowercase().as_str() {
        "base64" => {
            let clean: Vec<u8> = body.iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
            B64.decode(&clean).unwrap_or_else(|_| body.to_vec())
        }
        "quoted-printable" => qp_decode(body, false),
        _ => body.to_vec(),
    }
}

/// Quoted-printable: `=XX` is a byte, `=` at a line end is a soft break. In an encoded word
/// (`underscore_is_space`), `_` stands for a space.
pub(super) fn qp_decode(input: &[u8], underscore_is_space: bool) -> Vec<u8> {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let c = input[i];
        if c == b'=' {
            if input.get(i + 1) == Some(&b'\r') && input.get(i + 2) == Some(&b'\n') {
                i += 3;
                continue;
            }
            if input.get(i + 1) == Some(&b'\n') {
                i += 2;
                continue;
            }
            if let (Some(h), Some(l)) = (input.get(i + 1).and_then(|&b| hex(b)), input.get(i + 2).and_then(|&b| hex(b))) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(if underscore_is_space && c == b'_' { b' ' } else { c });
        i += 1;
    }
    out
}
