
/// What the model is told when its answer cites a link it never opened.
///
/// Unlike the other refusals, this one arrives with the work already done, so it
/// says what to do with the sentence rather than which tool to call next. Offered
/// exactly once a turn: a model that cites the same link twice will not be talked
/// out of it by a third request, and the user is better served by the answer with a
/// warning on it than by waiting on a loop.
pub fn citation_rebuke(urls: &[String]) -> String {
    format!(
        "Stop — your answer links to {} this turn never opened: {}. You do not know what is on that \
         page, so anything you said about it is invented, and a link the user clicks and finds wrong \
         costs them more than a shorter answer would have. Either open it now with `web_fetch` and \
         say what it actually says, or take it out. Then give the whole answer again.",
        if urls.len() == 1 { "a page" } else { "pages" },
        urls.iter().map(|u| format!("`{u}`")).collect::<Vec<_>>().join(", ")
    )
}

/// The half of the gates the model reads BEFORE it acts.
///
/// The refusals above are the backstop, and a backstop is expensive: a blocked call
/// costs a whole step, and the model has to work out what to do instead with less
/// context than it had when it chose. Saying the rule up front is cheaper than
/// enforcing it afterwards — and the two have to agree, which is why this text lives
/// next to the code that enforces it rather than in a prompt file that can drift
/// away from it.
///
/// Deliberately short and specific. "Ask if anything is unclear" changes no
/// behaviour, because nothing looks unclear from the inside; naming the moments
/// where guessing is expensive does. Each block is keyed to a tool the run actually
/// has, so nothing is told about a capability it hasn't got.
pub fn prompt_block(can_ask: bool, can_chart: bool, can_fetch: bool, can_read: bool, can_recall: bool) -> String {
    let mut s = String::new();
    if can_read {
        // Measured, not supposed: on the held-out split gemma4:12b answered a question
        // about a field its source did not contain by naming a value the source DID
        // contain, three times out of three. The feedback set never saw it — that model
        // admits a whole missing file readily and cites what it read correctly.
        // Admitting a missing *field inside a file it just read* is a different
        // property, and nothing was asking for it.
        //
        // A rule rather than a gate, deliberately. The checkable version — "the answer
        // names a term absent from every source" — fires on synonyms, and a file that
        // says `Maintainer:` when the user asked about the owner is a correct answer,
        // not a fabrication. Rule 1 of this module is that a gate which can fire on
        // truth is worse than no gate. So this steers, and the scoreboard says whether
        // the steering worked.
        s.push_str(concat!(
            "\n- If what you were asked for is not in the source, say it is not there. Do not answer",
            " with the nearest thing you did find — a config that sets `retries` and no timeout means",
            " there is no timeout, not that `retries` is one. \"That file does not say\" is a complete",
            " answer and it is the one the reader can act on; a confident substitution is the single",
            " error they have no way to catch.",
        ));
    }
    if can_ask {
        s.push_str(concat!(
            "\n- If the request has two sensible readings, `ask_user` and wait. It is a real question:",
            " the answers come back as the tool's result. Asked early, it beats work that has to be",
            " thrown away.",
            "\n- Every question must arrive with 2-4 answers you think are likely, each with a line on",
            " what choosing it would mean. A question with no options is refused — writing them is how",
            " you work out what the answers could be, and it lets the user reply in one click.",
            "\n- Ask about JUDGMENT, look up FACT. Which approach they want, what to prioritise, what a",
            " term means in their world: ask. A file path, a filename, a sheet or a column: find it",
            " (`list_dir`, `sheet_profile`, `grep`). Asking for something you could have looked up is",
            " asking the user to do your reading, and inventing it is worse.",
            "\n- Before producing a deliverable the user didn't specify the shape of — a chart, a report,",
            " a new file — check the shape is what they want, unless it is obvious.",
        ));
    }
    if can_recall {
        // BRAIN-RESEARCH-2026 §14. The rule was stated only in `recall_memory`'s own
        // result, which is the wrong place and the wrong time: the model reads it
        // after it already has both versions in front of it, and on `llama3.2` it
        // then either named the superseded person or refused to choose, 0/3. Stated
        // here for the same reason every other rule in this block is — a refusal
        // mid-run costs a step and arrives with less context than the choice did.
        //
        // A rule, not a gate. "The answer names a superseded entry" fires on the
        // honest sentence *"it was Dana, it is now Marcus"*, and rule 1 of this
        // module is that a gate which can fire on truth is worse than no gate.
        s.push_str(concat!(
            // Scoped to *memories* in so many words. The first version said "the LATER
            // one is what is true now" with nothing to bound it, and it is in the prompt
            // on every turn for any persona holding `recall_memory` — which put a
            // standing preference for later dates in front of `compare_two_files`, a task
            // that asks which of two dates is EARLIER. It fell 3/3 to 1/3, both failures
            // "picked the later date". A rule stated to steer one decision was steering
            // a different one.
            "\n- When `recall_memory` returns two memories that disagree about the same subject, trust",
            " the one recorded more recently; the older one is history. Say the current answer plainly",
            " rather than offering both, and mention the old one only if the user asked how things",
            " changed. Recall marks the entry it has superseded; do not answer from a marked one. This",
            " is only about which memory to believe — it has nothing to do with dates inside a question",
            " you are asked, where \"earlier\" and \"later\" mean exactly what they say.",
            "\n- If a memory names something you have not recalled on — a team, a person, a project —",
            " and the answer depends on it, call `recall_memory` again with that name. Two memories",
            " often hold half an answer each.",
        ));
    }
    if can_chart {
        s.push_str(concat!(
            "\n- Every number you chart must be one you actually read this turn. `chart` requires a",
            " `source`, and naming a file you have not opened is refused. A chart is read as evidence;",
            " invented figures in one are the worst output this app can produce.",
        ));
    }
    if can_fetch {
        s.push_str(concat!(
            "\n- Only link to pages you actually opened this turn. A plausible-looking URL you did not",
            " fetch is a fabricated source, and it is checked before your answer reaches the user.",
        ));
        // Provenance guidance. Its lesson was an assistant that
        // looped on a government 404 hunting for a threshold it could have marked as
        // unverified. Scoped to the WEB in so many words: the recall rule above tells
        // the model to take a second memory hop, and a general "stop searching" would
        // be the recency rule's mistake again — a sentence written for one decision
        // steering another.
        s.push_str(concat!(
            "\n- A figure, date, deadline or threshold you state as fact should come from something you",
            " read this turn, and you should be able to say which. If it comes from what you already know,",
            " say so. If you can't find a source on the web after a reasonable look, say it is unverified",
            " rather than searching until something turns up — an unsourced answer that says so is honest,",
            " and a search that never ends is an answer that never arrives.",
        ));
    }
    if s.is_empty() {
        return s;
    }
    format!("\n\n## Before you build the wrong thing\n{s}\n")
}
