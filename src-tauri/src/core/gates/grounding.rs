use super::*;

/// Domains reserved by RFC 2606 for documentation. A model reaching for one has
/// filled in an example, not a destination — there is nothing at the other end to
/// be right about.
pub(super) const RESERVED_HOSTS: &[&str] = &["example.com", "example.org", "example.net", "example.edu"];

/// Fragments that only ever appear in a value a model filled in for itself.
///
/// Deliberately short. Every entry here is a phrase no real file on a real machine
/// is called; the tempting additions (`data.csv`, `sample.xlsx`, `test.json`) are
/// left out precisely because they are also what people name real files.
pub(super) const PLACEHOLDER_FRAGMENTS: &[&str] = &[
    "path/to/",
    "path\\to\\",
    "/your_",
    "/your-",
    "your_file",
    "your-file",
    "yourfile",
    "your_data",
    "your-data",
    "/username/",
    "yourname",
    "<filename",
    "<path",
    "<file",
    "<url",
    "{filename",
    "{path",
    "{{",
    "[filename",
    "[your ",
    "replace_with",
    "replace-with",
    "insert_",
    "todo_fill",
];

/// Arguments whose value names a thing in the world, so a made-up one is a lie
/// rather than a style choice.
pub(super) const LOCATOR_KEYS: &[&str] = &["path", "url", "file", "filename", "source_path"];

/// Does `value` look like something the model filled in rather than something that
/// exists? `said` is the user's own words this turn: a value the user typed is never
/// a placeholder, whatever it looks like, which is what keeps "fetch example.com"
/// working when someone actually asks for it.
pub fn looks_invented(value: &str, said: &str) -> bool {
    let v = value.trim();
    if v.is_empty() {
        return false; // an empty argument is a different complaint, made elsewhere
    }
    if said.to_lowercase().contains(&v.to_lowercase()) {
        return false;
    }
    let low = v.to_lowercase();
    if PLACEHOLDER_FRAGMENTS.iter().any(|f| low.contains(f)) {
        return true;
    }
    // Host check, not substring: `example.com.mycompany.io` is somebody's real host,
    // and `notexample.com` is not the reserved one either.
    if let Some(host) = host_of(&low) {
        let host = host.strip_prefix("www.").unwrap_or(host);
        if RESERVED_HOSTS.contains(&host) {
            return true;
        }
    }
    false
}

pub(super) fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next()?;
    Some(host.split(':').next().unwrap_or(host))
}

/// The placeholder gate: refuse a tool call whose locator argument was invented.
///
/// Returns the message the model gets back in place of a result.
pub fn placeholder(name: &str, args: &Value, said: &str) -> Option<String> {
    let obj = args.as_object()?;
    for key in LOCATOR_KEYS {
        let Some(v) = obj.get(*key).and_then(Value::as_str) else { continue };
        if looks_invented(v, said) {
            return Some(format!(
                "error: `{key}` is \"{v}\", which is a placeholder, not a real location — \
                 nobody named it and it is not something this machine has. \
                 Do not fill one in. Find the real one with `list_dir`, or use `ask_user` to \
                 ask which {} you mean, then call `{name}` again with the answer.",
                if *key == "url" { "page" } else { "file" }
            ));
        }
    }
    None
}

/// Anything in `source` that names a locatable thing: a filename with an extension,
/// or a URL. Prose is ignored — the point is to check the checkable half.
pub fn cited_locators(source: &str) -> Vec<String> {
    source
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '"' | '\'' | '(' | ')'))
        .map(|t| t.trim_matches(|c: char| matches!(c, '.' | ':' | '`' | '*')))
        .filter(|t| {
            if t.starts_with("http://") || t.starts_with("https://") {
                return true;
            }
            // `q3.xlsx` yes, `Q3 revenue` no, `3.14` no, `etc.` no.
            match t.rsplit_once('.') {
                Some((stem, ext)) => {
                    !stem.is_empty()
                        && (2..=5).contains(&ext.len())
                        && ext.chars().all(|c| c.is_ascii_alphanumeric())
                        && !ext.chars().all(|c| c.is_ascii_digit())
                }
                None => false,
            }
        })
        .map(str::to_string)
        .collect()
}

/// The grounding gate: a chart must say where its numbers came from, and anything
/// it names must be something this run actually opened.
///
/// This is the gate the whole module exists for. A chart is the most trusted thing
/// an agent can produce — it is read as measurement, not as prose — so it is also
/// the most expensive place for invented numbers to land. `seen` is what this run
/// has genuinely read (files, sheets, pages); a `source` naming anything outside it
/// is a citation to something never opened.
///
/// Prose sources pass on purpose. "the figures in your message" is a true and
/// unverifiable answer, and the honest thing is to record it rather than to demand
/// a filename that would be the lie.
pub fn grounding(source: &str, seen: &[String]) -> Option<String> {
    let source = source.trim();
    if source.is_empty() {
        return Some(
            "error: `source` is required — say where these numbers came from (the file you read, \
             the page you fetched, or the figures the user gave you). A chart is read as evidence, \
             so it has to be traceable to something. If you have not read the data yet, read it \
             first with `sheet_read` or `read_file`."
                .into(),
        );
    }
    let cited = cited_locators(source);
    if cited.is_empty() {
        return None;
    }
    let missing: Vec<&String> = cited
        .iter()
        .filter(|c| {
            let c = c.to_lowercase();
            !seen.iter().any(|s| {
                let s = s.to_lowercase();
                // Either direction: `seen` holds the path as resolved, `source` may
                // name it as the user does (`q3.xlsx` for `reports/q3.xlsx`).
                s.contains(&c) || c.contains(&s)
            })
        })
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(format!(
        "error: `source` cites {} — this run never read {}. Do not chart numbers you have not read: \
         a chart of plausible figures is worse than no chart, because it looks like evidence. \
         Read {} first (`sheet_read`, `sheet_profile` or `read_file`){}.",
        missing.iter().map(|m| format!("\"{m}\"")).collect::<Vec<_>>().join(", "),
        if missing.len() == 1 { "it" } else { "them" },
        if missing.len() == 1 { "it" } else { "them" },
        if seen.is_empty() {
            ", or use `ask_user` to ask where the data actually is".to_string()
        } else {
            format!(". What this run HAS read: {}", seen.join(", "))
        }
    ))
}

/// How many identical failing calls before the gate stops the model repeating itself.
///
/// Three, not two: the second attempt is often a legitimate retry after something
/// transient. The third is a loop, and a loop burns the step budget that the rest of
/// the task needs.
pub const REPEAT_LIMIT: u32 = 3;

/// A stable identity for "this exact call", so repeats can be counted.
pub fn signature(name: &str, args: &Value) -> String {
    format!("{name}\u{1}{}", canonical(args))
}
