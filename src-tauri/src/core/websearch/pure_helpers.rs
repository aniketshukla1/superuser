//! pure helpers (unit-tested)

use super::*;

pub(super) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                if let Some(b) = hex { out.push(b); i += 3; } else { out.push(b'%'); i += 1; }
            }
            b'+' => { out.push(b' '); i += 1; }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Read an attribute value out of a tag's attribute string.
pub(super) fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// DuckDuckGo wraps result links in a /l/?uddg=<encoded> redirect — unwrap it.
pub(super) fn clean_url(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let after = &href[idx + 5..];
        let enc = after.split('&').next().unwrap_or(after);
        return percent_decode(enc);
    }
    if let Some(stripped) = href.strip_prefix("//") {
        return format!("https://{stripped}");
    }
    href.to_string()
}

pub fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#x27;", "'").replace("&#39;", "'").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse DuckDuckGo HTML results (the `result__a` / `result__snippet` markup).
pub fn parse_results(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results: Vec<SearchResult> = Vec::new();
    for seg in html.split("<a ").skip(1) {
        let gt = match seg.find('>') { Some(i) => i, None => continue };
        let tag = &seg[..gt];
        if !tag.contains("result__a") { continue; }
        let url = clean_url(&attr(tag, "href").unwrap_or_default());
        if url.is_empty() || !url.starts_with("http") { continue; }
        let title = seg[gt + 1..].split("</a>").next().map(strip_tags).unwrap_or_default();
        results.push(SearchResult { title, url, snippet: String::new() });
        if results.len() >= limit { break; }
    }
    let snippets: Vec<String> = html
        .split("result__snippet")
        .skip(1)
        .filter_map(|seg| {
            let gt = seg.find('>')?;
            let rest = &seg[gt + 1..];
            let end = rest.find("</a>").or_else(|| rest.find("</div>"))?;
            Some(strip_tags(&rest[..end]))
        })
        .collect();
    for (i, r) in results.iter_mut().enumerate() {
        if let Some(s) = snippets.get(i) { r.snippet = s.clone(); }
    }
    results
}

/// Strip a full HTML page to readable text (drops script/style/head).
pub fn html_to_text(html: &str) -> String {
    let mut s = html.to_string();
    for (open, close) in [
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<head", "</head>"),
        ("<nav", "</nav>"),
        ("<header", "</header>"),
        ("<footer", "</footer>"),
        ("<aside", "</aside>"),
        ("<form", "</form>"),
        ("<svg", "</svg>"),
        ("<!--", "-->"),
    ] {
        loop {
            let lower = s.to_lowercase();
            if let Some(a) = lower.find(open) {
                if let Some(b) = lower[a..].find(close) {
                    s.replace_range(a..a + b + close.len(), " ");
                    continue;
                }
            }
            break;
        }
    }
    strip_tags(&s)
}
