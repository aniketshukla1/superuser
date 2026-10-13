use super::*;

/// Recognise a cut by its exact shape — a head of two fifths of what was kept, the notice, the
/// tail — with a notice whose numbers add up and, given a store, a handle it holds. An output
/// that merely quotes a notice fails one of those and is cut as the output it is.
pub(super) fn prior_cut<'a>(s: &'a str, store: Option<&crate::core::observations::ObservationStore>) -> Option<PriorCut<'a>> {
    const OPEN: &str = "\n\n… [";
    const CLOSE: &str = "] …\n\n";
    let mut total_chars = None;
    let (mut from, mut counted_to, mut chars_before) = (0, 0, 0);
    while let Some(i) = s[from..].find(OPEN) {
        let at = from + i;
        from = at + 1;
        chars_before += s[counted_to..at].chars().count();
        counted_to = at;
        let body = at + 2;
        let Some(j) = s[body..].find(CLOSE) else { break };
        let end = body + j + "] …".len();
        let Some((cut, handle)) = read_notice(&s[body..end]) else { continue };
        let total = *total_chars.get_or_insert_with(|| s.chars().count());
        let head_chars = chars_before;
        let Some(tail_chars) = total.checked_sub(head_chars + 4 + s[body..end].chars().count()) else { continue };
        let kept = head_chars + tail_chars;
        let Some(whole) = kept.checked_add(cut) else { continue };
        if cut == 0 || head_chars != kept * 2 / 5 {
            continue;
        }
        if let Some((id, stored)) = &handle {
            let held = match store {
                Some(st) => st.has(id),
                None => crate::core::observations::well_formed(id),
            };
            if *stored != whole || !held {
                continue;
            }
        }
        return Some(PriorCut { head: &s[..at], tail: &s[end + 2..], head_chars, tail_chars, whole, handle });
    }
    None
}

/// A cut, cut again. It keeps what cutting the whole result at `limit` would keep — a smaller
/// limit keeps less of each end, so the new ends lie inside the old ones — and the notice keeps
/// the first cut's handle and the whole result's length, so nothing is stored twice and a recall
/// still returns the output. A limit that already holds what was kept changes nothing: the notice
/// is not content to cut.
pub(super) fn recut(s: &str, prior: &PriorCut, limit: usize) -> String {
    if prior.head_chars + prior.tail_chars <= limit {
        return s.to_string();
    }
    let head = (limit * 2 / 5).min(prior.head_chars);
    let tail = (limit - limit * 2 / 5).min(prior.tail_chars);
    let head_s: String = prior.head.chars().take(head).collect();
    let tail_s: String = prior.tail.chars().skip(prior.tail_chars - tail).collect();
    let notice = cut_notice(prior.whole - head - tail, prior.handle.as_ref().map(|(id, total)| (id.as_str(), *total)));
    format!("{head_s}\n\n{notice}\n\n{tail_s}")
}

/// True if this message is a tool result in either provider's shape.
pub(super) fn is_tool_result(msg: &Value) -> bool {
    if msg["role"] == "tool" {
        return true;
    }
    msg["role"] == "user"
        && msg["content"]
            .as_array()
            .is_some_and(|blocks| blocks.iter().any(|b| b["type"] == "tool_result"))
}

/// A turn starts at a real user message. Claude's tool results also ride on the
/// `user` role, so a turn boundary is a user message whose content is plain text —
/// counting tool results as turns would trim mid-tool-call and break the pairing.
pub(super) fn starts_turn(msg: &Value) -> bool {
    msg["role"] == "user" && msg["content"].is_string()
}

/// Rewrite over-long tool results in `msgs[..protect_from]` to head+tail. The
/// messages from `protect_from` on are the step the model is working through now
/// and are left whole. Returns how many results were shortened.
pub fn truncate_tool_results(
    msgs: &mut [Value],
    protect_from: usize,
    limit: usize,
    store: Option<&crate::core::observations::ObservationStore>,
) -> usize {
    let end = protect_from.min(msgs.len());
    let mut n = 0;
    for msg in msgs[..end].iter_mut() {
        if !is_tool_result(msg) {
            continue;
        }
        // Counted only when the text changed: a result cut on an earlier pass is over the limit
        // by its notice alone, and is left as it is.
        if let Some(s) = msg["content"].as_str() {
            if s.chars().count() > limit {
                let cut = middle_out_recorded(s, limit, store);
                if cut != s {
                    msg["content"] = Value::String(cut);
                    n += 1;
                }
            }
            continue;
        }
        if let Some(blocks) = msg["content"].as_array_mut() {
            for b in blocks.iter_mut() {
                if b["type"] != "tool_result" {
                    continue;
                }
                if let Some(s) = b["content"].as_str() {
                    if s.chars().count() > limit {
                        let cut = middle_out_recorded(s, limit, store);
                        if cut != s {
                            b["content"] = Value::String(cut);
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    n
}

/// Index where the most recent tool-calling step begins — the last `assistant`
/// message. Everything from there on is the exchange the model is reasoning about
/// right now and must stay whole; everything before it is fair game to shrink.
/// Falls back to the end of the list (protect nothing new) when there is no
/// assistant message yet, i.e. on the first step of a turn.
pub fn last_step_start(msgs: &[Value]) -> usize {
    msgs.iter().rposition(|m| m["role"] == "assistant").unwrap_or(msgs.len())
}

/// Squeeze `msgs` under `max_tokens` by truncating historical tool results ever
/// harder — full limit first, then halving down to `MIN_TOOL_RESULT_CHARS`. No
/// message is ever removed, so `tool_use`/`tool_result` pairing survives.
///
/// Returns the number of results shortened. A run can still exceed the budget if
/// the bulk is prose rather than tool output; that's the caller's cue that the
/// history itself needs trimming, which happens before the loop starts.
pub fn relieve_pressure(
    msgs: &mut [Value],
    protect_from: usize,
    max_tokens: usize,
    store: Option<&crate::core::observations::ObservationStore>,
) -> usize {
    let mut shortened = 0;
    let mut limit = TOOL_RESULT_CHAR_LIMIT;
    loop {
        shortened += truncate_tool_results(msgs, protect_from, limit, store);
        if estimate_messages(msgs) <= max_tokens || limit <= MIN_TOOL_RESULT_CHARS {
            return shortened;
        }
        limit = (limit / 2).max(MIN_TOOL_RESULT_CHARS);
    }
}
