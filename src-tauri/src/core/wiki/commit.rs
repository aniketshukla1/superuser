use super::*;

pub(super) fn parse_pattern(id: &str, text: &str) -> Pattern {
    let (front, body) = match split_frontmatter(text) {
        Some(x) => x,
        None => (Front::default(), text.trim().to_string()),
    };
    Pattern {
        id: id.to_string(),
        title: front.title.unwrap_or_else(|| id.replace('-', " ")),
        kind: front.kind.as_deref().map(Kind::parse).unwrap_or(Kind::Failure),
        // A page whose counter is missing has still been seen once — it exists.
        seen: front.seen.unwrap_or(1).max(1),
        first_seen: front.first_seen.clone().unwrap_or_default(),
        updated: front.updated.or(front.first_seen).unwrap_or_default(),
        body,
    }
}

pub(super) fn split_frontmatter(text: &str) -> Option<(Front, String)> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = t.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let front: Front = serde_yaml_ng::from_str(&rest[..end]).unwrap_or_default();
    Some((front, rest[end + 4..].trim().to_string()))
}

/// Split a markdown table row into unescaped cells, or `None` if it isn't one.
pub(super) fn row_cells(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with('|') || !t.ends_with('|') || t.len() < 2 {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    // Split on pipes that were not escaped when the row was written.
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                cur.push('|');
            }
            '|' => cells.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    cells.push(cur);
    Some(cells.into_iter().map(|c| c.trim().to_string()).collect())
}

pub(super) fn parse_impact_row(line: &str) -> Option<Impact> {
    let c = row_cells(line)?;
    if c.len() < 6 {
        return None;
    }
    let verdict = match c[3].as_str() {
        "undone" => Verdict::Undone,
        "accepted" => Verdict::Accepted,
        // Header and separator rows land here, and so does a row a human has
        // mangled. None of them is an impact.
        _ => return None,
    };
    Some(Impact {
        at: c[0].clone(),
        target: c[1].clone(),
        patterns: c[2].split_whitespace().map(str::to_string).collect(),
        verdict,
        snapshot: c[4].clone(),
        note: c[5].clone(),
    })
}

pub(super) fn replace_verdict(line: &str, v: Verdict) -> String {
    let Some(cells) = row_cells(line) else { return line.to_string() };
    let mut out = cells;
    out[3] = v.as_str().to_string();
    format!("| {} |", out.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "))
}

/// Everything a pass wants to remember, applied together so a partial write cannot
/// leave the index disagreeing with the pages.
pub fn commit(
    vault: &Path,
    patterns: &[(String, String, Kind, String)],
    impacts: &[Impact],
    log_line: &str,
) -> Vec<String> {
    let wiki = Wiki::new(vault);
    let mut written = Vec::new();
    for (id, title, kind, body) in patterns {
        if body.trim().is_empty() {
            continue;
        }
        if wiki.upsert_pattern(id, title, *kind, body).is_ok() {
            written.push(Wiki::safe_id(id));
        }
    }
    for im in impacts {
        let _ = wiki.record_impact(im);
    }
    if !written.is_empty() {
        let _ = wiki.rewrite_index();
    }
    if !log_line.trim().is_empty() {
        let _ = wiki.log(log_line);
    }
    written
}

/// Provenance for a skill the reviewer changed: which patterns motivated it.
///
/// Written by the harness rather than by the reviewer, for the same reason the
/// `seen` counter is: a model asked to also emit its own citations will sometimes
/// emit a citation and no edit. This file lives under `skills/`, so it rolls back
/// with the skill it explains — an undone edit should not leave behind a note
/// claiming a reason for a change that is no longer there.
pub fn write_purpose(skill_dir: &Path, skill: &str, patterns: &[String], note: &str) -> io::Result<()> {
    if patterns.is_empty() {
        return Ok(());
    }
    let body = format!(
        "# Why `{skill}` looks like this\n\nLast changed {} by a self-learning review.\n\n{}\n\nMotivating patterns:\n{}\n",
        today(),
        if note.trim().is_empty() { "_No note given._" } else { note.trim() },
        patterns.iter().map(|p| format!("- `{p}` — see `journal/wiki/patterns/{p}.md`")).collect::<Vec<_>>().join("\n"),
    );
    std::fs::create_dir_all(skill_dir)?;
    write_atomic(&skill_dir.join("PURPOSE.md"), &body)
}

/// A read-only view for the UI: the whole wiki, small enough to ship in one response.
pub fn overview(vault: &Path) -> BTreeMap<String, serde_json::Value> {
    let wiki = Wiki::new(vault);
    let mut m = BTreeMap::new();
    m.insert("patterns".into(), serde_json::to_value(wiki.patterns()).unwrap_or(serde_json::Value::Null));
    m.insert("impacts".into(), serde_json::to_value(wiki.impacts()).unwrap_or(serde_json::Value::Null));
    m.insert("log".into(), serde_json::Value::String(std::fs::read_to_string(wiki.logs_path()).unwrap_or_default()));
    m
}
