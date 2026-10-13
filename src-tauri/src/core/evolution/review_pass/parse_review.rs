use super::super::*;

/// Parse a review answer. Returns `None` when it asked for nothing at all — a
/// silent or unparseable response both mean "change nothing", which is the safe
/// reading.
pub fn parse_review(raw: &str) -> Option<Review> {
    if raw.contains(SILENT_TOKEN) {
        return None;
    }
    let edits: Vec<Edit> = blocks(raw, "edit")
        .into_iter()
        .filter_map(|(attrs, content)| {
            let path = attr(&attrs, "path")?;
            (!path.is_empty() && !content.trim().is_empty()).then_some(Edit { path, content })
        })
        .collect();

    let patterns: Vec<PatternEdit> = blocks(raw, "pattern")
        .into_iter()
        .filter_map(|(attrs, body)| {
            // No id means no stable page to reinforce next time, which is the only
            // thing a pattern is for. Dropped rather than given a generated name.
            let id = attr(&attrs, "id").filter(|i| !i.trim().is_empty())?;
            (!body.trim().is_empty()).then(|| PatternEdit {
                id: Wiki::safe_id(&id),
                title: attr(&attrs, "title").unwrap_or_default(),
                kind: wiki::Kind::parse(&attr(&attrs, "kind").unwrap_or_default()),
                body: body.trim().to_string(),
            })
        })
        .collect();

    if edits.is_empty() && patterns.is_empty() {
        return None;
    }

    let one = |tag: &str| {
        blocks(raw, tag).into_iter().next().map(|(_, b)| b.trim().to_string()).unwrap_or_default()
    };
    let because = one("because")
        .split([',', '\n'])
        .map(|x| Wiki::safe_id(x.trim()))
        .filter(|x| !x.is_empty() && x != "page")
        .collect();

    Some(Review { edits, patterns, because, note: one("note") })
}

/// The most new core-memory bullets one review may add. The prompt asks for one;
/// this makes two the hard ceiling.
pub(in crate::core::evolution) const MAX_NEW_CORE_BULLETS: usize = 2;

pub(in crate::core::evolution) fn bullets(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with('-') || l.starts_with('*'))
        .map(|l| l.trim_start_matches(['-', '*', ' ']).trim().to_string())
        .collect()
}

/// Core memory is append-only from a review's point of view.
///
/// Consolidating, merging and pruning it is Deep Dream's job, with a diary and its
/// own safety rails. A review that rewrites the file wholesale — which a small
/// model will occasionally do, especially off a thin conversation — would quietly
/// destroy long-term memory in a way nobody would notice until something the
/// assistant used to know went missing. So: existing bullets must all survive, and
/// at most a couple may be added.
pub fn guard_core_edit(before: &str, after: &str) -> Result<(), String> {
    let (old, new) = (bullets(before), bullets(after));
    if let Some(lost) = old.iter().find(|b| !new.contains(b)) {
        return Err(format!("refused: a review may not remove core memory (\"{}\")", lost.chars().take(60).collect::<String>()));
    }
    let added = new.len().saturating_sub(old.len());
    if added > MAX_NEW_CORE_BULLETS {
        return Err(format!("refused: {added} new core entries in one pass (max {MAX_NEW_CORE_BULLETS})"));
    }
    Ok(())
}

/// A written skill has to be a skill. A model that decides small talk deserves a
/// new skill tends to emit prose without frontmatter; that would sit in the skills
/// directory being silently skipped at load time, which is worse than not writing it.
pub fn guard_skill_edit(name: &str, content: &str) -> Result<(), String> {
    crate::core::skills::parse_skill(content, name, None)
        .map(|_| ())
        .map_err(|e| format!("refused: not a valid SKILL.md ({e})"))
}

/// Resolve a review-supplied path against the allowlist. Rejects absolute paths,
/// traversal, and anything that isn't one of the two writable shapes — the model
/// picks these strings, so they are untrusted input.
pub fn resolve_path(data_dir: &Path, vault: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().trim_start_matches("./");
    if rel.is_empty() || Path::new(rel).is_absolute() || rel.split(['/', '\\']).any(|c| c == ".." || c == "") {
        return Err(format!("refused path '{rel}'"));
    }
    if rel == "journal/MEMORY.md" {
        return Ok(Journal::new(vault).core_path());
    }
    // skills/<name>/SKILL.md — exactly three segments, nothing deeper.
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() == 3 && parts[0] == "skills" && parts[2] == "SKILL.md" && !parts[1].is_empty() {
        return Ok(data_dir.join("skills").join(parts[1]).join("SKILL.md"));
    }
    Err(format!("refused path '{rel}' — only skills/<name>/SKILL.md and journal/MEMORY.md are writable"))
}

/// The most patterns one brief will name. Past that the prompt starts crowding out
/// the transcript, and the patterns beyond it are the least-repeated ones.
pub(in crate::core::evolution) const BRIEF_PATTERNS: usize = 25;

/// What the review is shown: the conversation, the current content of the files it
/// may change, and what earlier passes learned. It has no tools, so anything it
/// must see has to be here — which also keeps the pass to a single model call.
///
/// The wiki goes *here* and nowhere else. WikiSkill's ablation is specific about
/// this: giving the wiki to the agent doing the work costs ~2.8 points, because an
/// assistant that can read the answers stops producing the failures the reviewer
/// needs to see. The reviewer reads it; the assistant never does.
pub(in crate::core::evolution) fn build_prompt(transcript: &str, data_dir: &Path, vault: &Path) -> String {
    let mut s = String::new();
    let journal = Journal::new(vault);
    let core = journal.read_core();
    s.push_str("## Current core memory (journal/MEMORY.md)\n\n");
    s.push_str(if core.trim().is_empty() { "(empty)" } else { core.trim() });

    s.push_str("\n\n## Editable skills\n\n");
    let mut any = false;
    if let Ok(rd) = std::fs::read_dir(data_dir.join("skills")) {
        for entry in rd.flatten() {
            let md = entry.path().join("SKILL.md");
            if let Ok(text) = std::fs::read_to_string(&md) {
                let name = entry.file_name().to_string_lossy().to_string();
                s.push_str(&format!("### skills/{name}/SKILL.md\n\n{}\n\n", crate::core::context::middle_out(&text, 4_000)));
                any = true;
            }
        }
    }
    if !any {
        s.push_str("(none yet — you may create one if a genuinely reusable workflow emerged)\n\n");
    }
    let brief = Wiki::new(vault).brief(BRIEF_PATTERNS);
    if !brief.trim().is_empty() {
        s.push_str(&crate::core::context::middle_out(&brief, 6_000));
        s.push_str("\n\n");
    }

    s.push_str("## The conversation that just went idle\n\n<transcript>\n");
    s.push_str(&crate::core::context::middle_out(transcript, 12_000));
    s.push_str("\n</transcript>\n");
    s
}
