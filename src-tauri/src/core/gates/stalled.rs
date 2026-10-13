use super::*;

/// Key-ordered JSON, so argument order can't disguise a repeat as a new call.
pub(super) fn canonical(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys.iter().map(|k| format!("{k}:{}", canonical(&m[*k]))).collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(a) => format!("[{}]", a.iter().map(canonical).collect::<Vec<_>>().join(",")),
        other => other.to_string(),
    }
}

/// The repetition gate: the same call, failing the same way, is not worth a third run.
///
/// `failures` is how many times this exact call has already failed in this run.
pub fn repetition(name: &str, failures: u32) -> Option<String> {
    if failures + 1 < REPEAT_LIMIT {
        return None;
    }
    Some(format!(
        "error: you have already called `{name}` with these exact arguments {failures} times and it \
         failed every time. Running it again will fail again. Change something: different arguments, \
         a different tool, or `ask_user` to ask the user what you are missing. If you cannot proceed, \
         say so plainly instead of retrying."
    ))
}

/// How many locator misses in a row before the model is made to look.
///
/// Two. The first wrong path is an honest guess; the second, without having looked
/// in between, is a habit.
pub const GROPE_LIMIT: u32 = 2;

/// The groping gate: guessing at paths instead of listing them.
///
/// [`repetition`] catches the call repeated with *identical* arguments. A model
/// hunting for a file does not repeat itself — it guesses `reports/q3.csv`, then
/// `data/q3.csv`, then `q3_final.csv` — so every one of those is a fresh signature
/// and the existing gate never fires.
///
/// Measured on this machine's own session logs (TOOLS-RESEARCH-2026 §11):
/// `read_file` was called nine times across five sessions and failed nine times,
/// every one an invented path, while `list_dir` was called once and `grep` never in
/// fifty-nine sessions. The wider literature has the same shape from the other end —
/// on OSWorld, 66% of the steps in failed tasks are spent in a grounding loop.
///
/// The counter is reset by any call that *looks* (`list_dir`, `grep`, `glob`), so
/// the honest sequence — guess, miss, list, read — never trips it. What trips it is
/// the second miss with no look in between, and the refusal names the tool that
/// ends it. This gate can only ever fire after two failures the run already paid
/// for; it costs nothing when the first guess is right.
pub fn groping(name: &str, misses: u32) -> Option<String> {
    if misses < GROPE_LIMIT {
        return None;
    }
    Some(format!(
        "error: that is {misses} paths in a row that do not exist, and nothing has listed the \
         directory in between. Stop guessing: call `list_dir` on the folder you think it is in, or \
         `grep` for a string you expect the file to contain, and read the name off the result. \
         Then call `{name}` with a path you have actually seen."
    ))
}

/// Does this tool call look at what exists, rather than assuming it?
pub fn is_a_look(name: &str) -> bool {
    matches!(name, "list_dir" | "grep" | "git_status")
}

/// Did this result say the locator was not there?
///
/// Deliberately narrow: a missing path, not any error. A permissions failure or a
/// binary-format complaint is not groping and must not be counted as it.
pub fn is_a_miss(result: &str) -> bool {
    let low = result.trim_start().to_lowercase();
    low.starts_with("error") && (low.contains("no such path") || low.contains("couldn't open"))
}

/// How many identical results in a row before a call is told it is going nowhere.
///
/// Two is the answer, and it is deliberately lower than [`REPEAT_LIMIT`]. A failing
/// call might succeed on a retry — the file might appear, the server might come back
/// — so three is a fair number of tries. A call that has twice returned *byte-for-byte
/// the same thing* has no such excuse: the third run is arithmetic, not hope.
pub const STALL_LIMIT: u32 = 2;

/// The stall gate: a call that keeps working and keeps saying the same thing.
///
/// [`repetition`] above catches the call that keeps failing. This catches the one
/// nothing was watching — the call that succeeds every time and moves nothing. In
/// the 2026-09-06 harness sweep one turn ran 22 steps over 1,036 seconds that way,
/// every call returning ok, and answered nothing. On a local model each of those
/// steps is a full prefill; seventeen minutes is not a slow answer, it is a hang.
///
/// `identical` is how many times in a row this exact call has returned this exact
/// output — see [`crate::core::agent_exec::ToolCtx::note_result`], which keys on the
/// result precisely so that a legitimate re-read (after an edit, after a write, after
/// a fix) never trips this. Only a call that has stopped adding information does.
///
/// The message names what the model has and what to do with it. "You are looping" is
/// true and useless; a model that had a next move would have made it, so this has to
/// suggest one — and give it permission to answer from what it already read, which is
/// usually the correct move and the one a stuck model will not take on its own.
pub fn stalled(name: &str, identical: u32) -> Option<String> {
    if identical < STALL_LIMIT {
        return None;
    }
    Some(format!(
        "error: `{name}` has already returned exactly this result {identical} times in this turn, and \
         nothing has changed since. Running it again will return the same thing a {} time. You already \
         have everything this call can tell you — it is in the conversation above. Either use it to \
         answer now, or do something genuinely different: a different tool, different arguments, or a \
         different approach entirely. If what you need is not there, say so plainly and stop; a wrong \
         answer and a long silence are both worse than saying what you could not find.",
        match identical + 1 {
            3 => "third".to_string(),
            4 => "fourth".to_string(),
            n => format!("{n}th"),
        }
    ))
}

/// Every URL in a piece of text, lowercased for comparison.
///
/// Trailing punctuation comes off: a URL at the end of a sentence ends in a full
/// stop that is not part of it, and one inside a markdown link ends in `)`.
pub fn urls_in(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while let Some(at) = text[i..].find("http") {
        let start = i + at;
        let tail = &text[start..];
        if !(tail.starts_with("http://") || tail.starts_with("https://")) {
            i = start + 4;
            continue;
        }
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`' | '|' | '\\'))
            .unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | ')' | ']' | '}' | '!' | '?'));
        // Shorter than "http://a" is not a URL, it is the word in prose.
        if url.len() > 8 {
            let url = url.to_lowercase();
            if !out.contains(&url) {
                out.push(url);
            }
        }
        i = start + end.max(1);
    }
    out
}

/// URLs an answer cites that this run never saw anywhere.
///
/// **Why URLs and not file paths.** The argument-level gates already cover files:
/// [`placeholder`] refuses an invented path on the way in, and [`grounding`] refuses
/// a chart citing a file this run never opened. What is left for prose is what
/// arguments cannot catch — a source named only in the answer. And prose is a far
/// worse place to pattern-match: `Node.js`, `numpy.array` and `self.value` are
/// indistinguishable from filenames by shape, so a path rule here would fire on
/// ordinary technical writing. A gate that cries wolf on correct prose is worse than
/// no gate, because it teaches everyone to ignore it. A URL is unambiguous.
///
/// `seen` is everything the run read or encountered — pages fetched, plus URLs that
/// came back inside tool results, because a link returned by a real search or found
/// on a real page demonstrably exists even if its contents were never read. `said`
/// is the user's own message: a URL they typed is theirs to cite.
pub fn uncited_urls(answer: &str, seen: &[String], said: &str) -> Vec<String> {
    let said = said.to_lowercase();
    let seen: Vec<String> = seen.iter().map(|s| s.to_lowercase()).filter(|s| s.len() > 8).collect();
    urls_in(answer)
        .into_iter()
        .filter(|u| {
            if said.contains(u.as_str()) {
                return false;
            }
            // Either direction: the run may hold `https://a.io/x` while the answer
            // cites `https://a.io/x#results`, or the other way about.
            !seen.iter().any(|s| s.contains(u.as_str()) || u.contains(s.as_str()))
        })
        .collect()
}
