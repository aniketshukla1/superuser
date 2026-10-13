use super::*;

/// Characters of a historical tool result kept before it is summarized to
/// head+tail. ~5k tokens: big enough for a real file or command output, small
/// enough that a dozen of them don't fill the window.
pub const TOOL_RESULT_CHAR_LIMIT: usize = 20_000;

/// Floor for the adaptive squeeze. Below this a result is too small to be worth
/// keeping in the loop at all, and we'd be destroying signal for a few hundred
/// tokens.
pub(super) const MIN_TOOL_RESULT_CHARS: usize = 1_000;

/// Rough token count: ~4 characters per token, the usual English approximation.
/// Deliberately an estimate — an exact tokenizer per provider would cost more than
/// the headroom it buys, and every decision here has slack built in.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Estimated tokens for a whole message, counting every string leaf of its JSON
/// (content, tool names, arguments) plus a small per-message framing overhead.
pub fn estimate_message(msg: &Value) -> usize {
    fn walk(v: &Value, acc: &mut usize) {
        match v {
            Value::String(s) => *acc += estimate_tokens(s),
            Value::Array(a) => a.iter().for_each(|x| walk(x, acc)),
            Value::Object(o) => o.values().for_each(|x| walk(x, acc)),
            _ => {}
        }
    }
    let mut acc = 4; // role + delimiters
    walk(msg, &mut acc);
    acc
}

pub fn estimate_messages(msgs: &[Value]) -> usize {
    msgs.iter().map(estimate_message).sum()
}

/// The effective budget for one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Input tokens the assembled conversation may occupy.
    pub max_tokens: usize,
    /// Past turns carried into the run.
    pub max_turns: usize,
    /// Tool-call steps before the model must answer.
    pub max_steps: usize,
}

impl Budget {
    /// `local` runs against Ollama, where `num_ctx` is the hard ceiling for input
    /// *and* output combined — exceeding it doesn't error, it silently drops the
    /// front of the conversation, which reads as the model "forgetting" the task.
    /// So the real budget is the lower of the configured one and what num_ctx
    /// leaves after reserving room for the reply.
    pub fn resolve(settings: &Settings, local: bool) -> Self {
        let configured = settings.agent_max_context_tokens as usize;
        let max_tokens = if local {
            let room = (settings.num_ctx as usize).saturating_sub(settings.max_tokens as usize);
            // A pathological num_ctx (smaller than the reply reservation) would floor
            // this at zero and trim away the user's own message. Keep a usable ceiling.
            configured.min(room).max(2_000)
        } else {
            configured
        };
        Self {
            max_tokens,
            max_turns: settings.agent_max_context_turns as usize,
            max_steps: settings.agent_max_steps as usize,
        }
    }

    /// The same budget, taken against the model's *real* window rather than against
    /// the local allocation setting alone.
    ///
    /// [`Budget::resolve`] can only ask "is this local?", so it applies `num_ctx` to
    /// every Ollama model equally and applies nothing at all to a cloud one. A
    /// profile knows the window, which is the difference between a 32k model asked to
    /// hold 128k and one that genuinely can — see
    /// [`ModelProfile::input_budget`](crate::core::model_profile::ModelProfile::input_budget).
    pub fn for_model(settings: &Settings, profile: &crate::core::model_profile::ModelProfile) -> Self {
        Self {
            max_tokens: profile.input_budget(settings),
            max_turns: settings.agent_max_context_turns as usize,
            max_steps: settings.agent_max_steps as usize,
        }
    }
}

/// Keep the head and tail of an over-long string, with a notice in between saying
/// how much was cut. Character-based, so it never splits a UTF-8 sequence.
///
/// Used directly for prose the model will never need to recover verbatim —
/// evolution briefs, journal evidence, transcript previews. Tool results inside a
/// turn go through [`middle_out_recorded`] instead, so the cut text stays
/// reachable.
pub fn middle_out(s: &str, limit: usize) -> String {
    middle_out_recorded(s, limit, None)
}

/// [`middle_out`], but the removed middle is kept and the notice names a handle.
///
/// The old notice said *"re-read the source if you need the middle"*. That is a
/// courtesy on a `read_file` result and a lie on a `run_shell` one: the ninety
/// seconds of build output, the `git log` on a tree that has moved, the MCP call
/// that will rate-limit the second time. Telling the model to re-fetch something
/// that no longer exists costs a step and then an answer.
///
/// With a store, the full text is written before the cut and the notice carries
/// `recall_output(id="<id>")` instead. Without one — or if the store refuses, which it does
/// at its cap — the wording falls back to the old sentence, because a handle that
/// resolves to nothing would be worse than no handle at all.
///
/// Text this function already cut is cut around what it kept, never as new output: see
/// `recut`. A cut is the limit plus its notice, so it is always a little over the limit, and
/// every later pass used to cut it again — the notice out, the previous cut stored as if it were
/// the output, and the handle one step further from the original each time.
pub fn middle_out_recorded(
    s: &str,
    limit: usize,
    store: Option<&crate::core::observations::ObservationStore>,
) -> String {
    if let Some(prior) = prior_cut(s, store) {
        return recut(s, &prior, limit);
    }
    let total = s.chars().count();
    if total <= limit {
        return s.to_string();
    }
    // Favour the tail: a command's error line and a file's closing structure are
    // usually more load-bearing than another screen of the middle.
    let head = limit * 2 / 5;
    let tail = limit - head;
    let head_s: String = s.chars().take(head).collect();
    let tail_s: String = s.chars().skip(total - tail).collect();
    let cut = total - head - tail;
    // Store the WHOLE result, not the excised middle: a recall should hand back
    // what the model saw, and reassembling head + middle + tail at the other end
    // is work the model should not have to do to read one command's output.
    let recorded = store.and_then(|st| st.put(s));
    let notice = cut_notice(cut, recorded.as_deref().map(|id| (id, total)));
    format!("{head_s}\n\n{notice}\n\n{tail_s}")
}

/// The notice a cut leaves: how much was cut and, when the whole result was stored, the handle
/// that returns it and its length. Written in one place because [`prior_cut`] reads it back.
pub(super) fn cut_notice(cut: usize, recorded: Option<(&str, usize)>) -> String {
    match recorded {
        Some((id, total)) => format!(
            "… [{cut} characters truncated to save context — recall_output(id=\"{id}\") for the whole {total}-character result, or recall_output(id=\"{id}\", from, len) for one window of it] …"
        ),
        None => format!("… [{cut} characters truncated to save context — re-read the source if you need the middle] …"),
    }
}

/// The numbers in a notice [`cut_notice`] wrote, or `None` for anything it didn't write: read
/// loosely, then accepted only if writing them again gives back the same notice exactly.
pub(super) fn read_notice(notice: &str) -> Option<(usize, Option<(String, usize)>)> {
    let rest = notice.strip_prefix("… [")?;
    let cut: usize = rest[..rest.find(|c: char| !c.is_ascii_digit())?].parse().ok()?;
    let recorded = match notice.split_once("recall_output(id=\"") {
        None => None,
        Some((_, after)) => {
            let id = after.split('"').next()?.to_string();
            let whole = notice.split_once("for the whole ")?.1;
            let total: usize = whole[..whole.find(|c: char| !c.is_ascii_digit())?].parse().ok()?;
            Some((id, total))
        }
    };
    (cut_notice(cut, recorded.as_ref().map(|(id, total)| (id.as_str(), *total))) == notice).then_some((cut, recorded))
}

/// A result [`middle_out_recorded`] already cut: the head and tail it kept, their lengths, the
/// whole result's length, and the stored whole's handle when there is one.
pub(super) struct PriorCut<'a> {
    pub(super) head: &'a str,
    pub(super) tail: &'a str,
    pub(super) head_chars: usize,
    pub(super) tail_chars: usize,
    pub(super) whole: usize,
    pub(super) handle: Option<(String, usize)>,
}
