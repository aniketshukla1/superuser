use super::*;

/// Apply a unique `old_str` → `new_str` replacement to a file's current content.
/// Returns (old_content, new_content) for a proposal, or an error if `old_str`
/// is missing or not unique — the str-replace contract used by agentic editors.
/// Carry the file's own spacing into a loosely matched edit.
///
/// The loose match finds `timeout = 30` for a model that asked to replace
/// `timeout=30`, and the first version then wrote the model's `timeout=60` over the
/// whole span — landing the edit and quietly changing the line's spacing. The task
/// said "leave everything else exactly as it is", and its check duly failed a file
/// reading `timeout=60`. The check was right.
///
/// So only the part the model actually changed is replaced. The common prefix and
/// suffix of the model's before-and-after locate the changed middle; that middle is
/// mapped onto the file's span by counting non-space characters, and everything
/// outside it keeps the file's own spacing. `timeout=30` → `timeout=60` against
/// `timeout = 30` yields `timeout = 60`.
///
/// Conservative: when either middle contains whitespace — the model deliberately
/// re-spaced something — its text is used as it was, which is the old behaviour.
pub(super) fn respace(model_old: &str, span: &str, model_new: &str) -> String {
    let o: Vec<char> = model_old.chars().collect();
    let n: Vec<char> = model_new.chars().collect();
    let mut p = 0;
    while p < o.len() && p < n.len() && o[p] == n[p] {
        p += 1;
    }
    let mut s = 0;
    while s < o.len() - p && s < n.len() - p && o[o.len() - 1 - s] == n[n.len() - 1 - s] {
        s += 1;
    }
    let old_mid: String = o[p..o.len() - s].iter().collect();
    let new_mid: String = n[p..n.len() - s].iter().collect();
    if old_mid.chars().any(char::is_whitespace) || new_mid.chars().any(char::is_whitespace) {
        return model_new.to_string();
    }
    let non_space_before = |upto: usize| o[..upto].iter().filter(|c| !c.is_whitespace()).count();
    let (k1, k2) = (non_space_before(p), non_space_before(o.len() - s));
    let offs: Vec<(usize, char)> = span.char_indices().filter(|(_, c)| !c.is_whitespace()).collect();
    let start = offs.get(k1).map(|(i, _)| *i).unwrap_or(span.len());
    let end = if k2 > k1 {
        offs.get(k2 - 1).map(|(i, c)| i + c.len_utf8()).unwrap_or(span.len())
    } else {
        start
    };
    if start > end || end > span.len() {
        return model_new.to_string();
    }
    format!("{}{}{}", &span[..start], new_mid, &span[end..])
}

/// The one span of `hay` equal to `needle` once every whitespace character is ignored.
///
/// `Err(0)` when there is no such span, `Err(n)` when there are `n` of them — the
/// loose match is only ever used when it is unambiguous. Needles shorter than three
/// non-space characters are refused outright: at that length ignoring spacing matches
/// too much to mean anything.
pub(super) fn loose_span(hay: &str, needle: &str) -> Result<String, usize> {
    let n: Vec<char> = needle.chars().filter(|c| !c.is_whitespace()).collect();
    if n.len() < 3 {
        return Err(0);
    }
    let kept: Vec<(usize, char)> = hay.char_indices().filter(|(_, c)| !c.is_whitespace()).collect();
    if kept.len() < n.len() {
        return Err(0);
    }
    let mut hits: Vec<usize> = Vec::new();
    for i in 0..=kept.len() - n.len() {
        if kept[i..i + n.len()].iter().map(|(_, c)| *c).eq(n.iter().copied()) {
            hits.push(i);
        }
    }
    match hits.as_slice() {
        [i] => {
            let start = kept[*i].0;
            let (last, c) = kept[*i + n.len() - 1];
            Ok(hay[start..last + c.len_utf8()].to_string())
        }
        many => Err(many.len()),
    }
}

pub fn str_replace(root: &str, rel: &str, old_str: &str, new_str: &str) -> Result<(String, String), String> {
    let full = Path::new(root).join(rel);
    let current = read(root, &full.to_string_lossy())?;

    // An empty `old_str` is how a model says "append", and it was refused. Read off the
    // traces rather than guessed: `append_keeps_the_rest` failed 3/3 on
    // `edit_file(old_str="", new_str="four")` → "old_str must not be empty", and the
    // model stopped there every time. There is exactly one sensible reading of
    // "replace nothing with this", and it is "add this" — so it now means that, by
    // rewriting the request into an ordinary whole-file match rather than growing a
    // second write path beside the one that already works.
    let (old_s, new_s): (String, String) = if old_str.is_empty() {
        if current.is_empty() {
            return Err(format!(
                "{rel} is empty, so there is nothing to add after — use write_file to give it its first contents."
            ));
        }
        let sep = if current.ends_with('\n') { "" } else { "\n" };
        let add = if new_str.ends_with('\n') { new_str.to_string() } else { format!("{new_str}\n") };
        (current.clone(), format!("{current}{sep}{add}"))
    } else {
        (old_str.to_string(), new_str.to_string())
    };
    let mut new_str: &str = new_s.as_str();

    // Tolerant of spacing, once, and only when the exact text is absent. Also read off
    // the traces: `edit_without_clobbering` sent `old_str="timeout=30"` for a file that
    // says `timeout = 30`, got "not found", and gave up. Matching with whitespace
    // ignored is what Aider and SWE-Edit's adaptive editors do; the guard that keeps it
    // safe is uniqueness — the loose match is used only when exactly one span fits, and
    // what is replaced is that span's real text, so nothing is edited that was not
    // there.
    let mut old_str: &str = old_s.as_str();
    let loose: String;
    let respaced: String;
    let mut occ = current.matches(old_str).count();
    if occ == 0 {
        match loose_span(&current, old_str) {
            Ok(span) => {
                // Change only what the model changed; the file keeps its own spacing
                // everywhere else. See `respace`.
                respaced = respace(old_s.as_str(), &span, new_str);
                new_str = respaced.as_str();
                loose = span;
                old_str = loose.as_str();
                occ = current.matches(old_str).count();
            }
            Err(0) => {
                return Err(format!(
                    "old_str not found in {rel}. read_file first and copy the exact text, including whitespace."
                ));
            }
            Err(n) => {
                return Err(format!(
                    "old_str matches {n} places in {rel} once spacing is ignored; include the lines above or \
                     below it so it matches exactly one."
                ));
            }
        }
    }
    if occ > 1 {
        // Say *where*, not just how many. "3 times" leaves the model to search for
        // the one it meant; "lines 14, 51, 90" turns the retry into a one-step fix
        // — the same reason the truncation notice names a handle instead of saying
        // the output was cut.
        let mut lines: Vec<usize> = Vec::new();
        let mut at = 0usize;
        while let Some(i) = current[at..].find(old_str) {
            let abs = at + i;
            lines.push(current[..abs].matches('\n').count() + 1);
            at = abs + old_str.len().max(1);
        }
        let shown: Vec<String> = lines.iter().take(6).map(|l| l.to_string()).collect();
        let more = lines.len().saturating_sub(shown.len());
        return Err(format!(
            "old_str appears {occ} times in {rel}, at line{} {}{}; include more surrounding context \
             — the lines above or below it — so it matches exactly once.",
            if shown.len() == 1 { "" } else { "s" },
            shown.join(", "),
            if more > 0 { format!(" and {more} more") } else { String::new() }
        ));
    }
    Ok((current.clone(), current.replacen(old_str, new_str, 1)))
}
