use super::super::*;

/// Where PROFILE.md lives.
///
/// Under `profile/` rather than at the vault root because `MemoryEngine::list`
/// scans the root non-recursively: a file there would be indexed as an ordinary
/// memory and could be recalled *back* into a prompt that already contains it
/// verbatim, which is how one preference turns into two copies of itself.
pub fn profile_md_path(vault: &Path) -> PathBuf {
    vault.join("profile").join("PROFILE.md")
}

pub(in crate::core::profile) const PROFILE_MD_HEADER: &str = "# Profile\n\n\
    How Superuser works with you. The blocks below are written by the app and are \
    replaced each time it learns something — anything you write outside them is left \
    alone, so this is a fine place to keep your own notes.\n";

pub(in crate::core::profile) fn begin(class: Class) -> String {
    format!("<!-- superuser:begin {} -->", class.slug())
}

pub(in crate::core::profile) fn end(class: Class) -> String {
    format!("<!-- superuser:end {} -->", class.slug())
}

/// Replace each class's managed block, appending the ones that aren't there yet.
///
/// Everything outside the markers is copied through untouched — that is the whole
/// contract. A class with nothing Active in it gets an empty block rather than
/// being deleted, so hand-written text that sits between two blocks doesn't shift
/// around every time a preference fades.
pub(in crate::core::profile) fn render_managed(existing: &str, grouped: &[(Class, Vec<Facet>)]) -> String {
    let mut out = existing.to_string();
    for class in CLASSES {
        let facets = grouped.iter().find(|(c, _)| *c == class).map(|(_, f)| f.as_slice()).unwrap_or(&[]);
        let body = if facets.is_empty() {
            String::new()
        } else {
            facets
                .iter()
                .map(|f| format!("- {}{}", f.value, if f.pinned { "  (pinned)" } else { "" }))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let block = format!(
            "{}\n## {}\n\n{}\n{}",
            begin(class),
            class.heading(),
            if body.is_empty() { "_Nothing learned yet._" } else { &body },
            end(class)
        );
        match (out.find(&begin(class)), out.find(&end(class))) {
            (Some(a), Some(b)) if b > a => {
                let b_end = b + end(class).len();
                out.replace_range(a..b_end, &block);
            }
            _ => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
                out.push_str(&block);
                out.push('\n');
            }
        }
    }
    out
}

pub(in crate::core::profile) fn decay(then: f64, now: f64, half_life_days: f64) -> f64 {
    let age_days = ((now - then) / DAY).max(0.0);
    0.5_f64.powf(age_days / half_life_days)
}

/// Flatten a value to one capped line.
///
/// Values reach the system prompt of every turn, so this is a boundary, not
/// tidying: no newlines (a value can't open its own section), no comment markers
/// (it can't forge a managed block), and a hard length cap (it can't become a
/// paragraph of instructions).
pub(in crate::core::profile) fn clean_value(s: &str) -> String {
    let flat: String = s
        .replace(['\n', '\r', '\t'], " ")
        .replace("<!--", "")
        .replace("-->", "")
        .trim()
        .trim_start_matches(['-', '*', '#', '>', ' '])
        .trim()
        .to_string();
    let mut squeezed = String::with_capacity(flat.len());
    let mut space = false;
    for c in flat.chars() {
        if c == ' ' {
            if !space {
                squeezed.push(c);
            }
            space = true;
        } else {
            squeezed.push(c);
            space = false;
        }
    }
    squeezed.chars().take(VALUE_MAX).collect::<String>().trim().to_string()
}

/// Normalize a key so two phrasings of one preference compete instead of both
/// rendering: `Preferred tone` and `preferred-tone` are the same slot.
pub(in crate::core::profile) fn clean_key(s: &str) -> String {
    let slug: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    slug.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-").chars().take(40).collect()
}

pub(in crate::core::profile) fn clean_evidence(s: &str) -> String {
    s.replace(['\n', '\r'], " ").trim().chars().take(240).collect()
}
