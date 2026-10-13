use super::*;

/// Balanced top-level `{…}` spans in `text`, string-aware, in order of appearance.
pub(super) fn json_objects(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'{' {
            i += 1;
            continue;
        }
        let (mut depth, mut in_str, mut esc, mut j) = (0i32, false, false, i);
        let mut end = None;
        while j < b.len() {
            let c = b[j];
            if in_str {
                if esc {
                    esc = false;
                } else if c == b'\\' {
                    esc = true;
                } else if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    end = Some(j + 1);
                    break;
                }
            }
            j += 1;
        }
        match end {
            Some(e) => {
                out.push(text[i..e].to_string());
                i = e;
            }
            None => break,
        }
    }
    out
}

pub(super) fn strict(text: &str, available: &[String]) -> Option<Value> {
    let mut body = text.trim();
    // ```json … ``` or ``` … ```
    if let Some(rest) = body.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        body = rest.trim_start_matches('\n').strip_suffix("```").unwrap_or(rest).trim();
    }
    if !(body.starts_with('{') && body.ends_with('}')) {
        return None;
    }
    let v: Value = serde_json::from_str(body).ok()?;
    let name = v["name"].as_str().or_else(|| v["tool"].as_str())?.trim().to_string();
    if !available.iter().any(|a| *a == name) {
        return None;
    }
    // `arguments`, `parameters`, `args` — all three are in the wild, and an absent
    // one is a call with no arguments rather than a reason to give up.
    let args = ["arguments", "parameters", "args", "input"]
        .iter()
        .map(|k| &v[*k])
        .find(|a| !a.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));
    // A string of JSON is as common as an object here.
    let args = match args.as_str() {
        Some(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
        None => args,
    };
    if !args.is_object() {
        return None;
    }
    Some(json!([{ "function": { "name": name, "arguments": args } }]))
}
