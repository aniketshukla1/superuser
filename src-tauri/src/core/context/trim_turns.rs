use super::*;

/// Split a plain `{role, content}` history into the turns that fit the budget and
/// the older ones that don't. Returns `(kept, dropped)`, both oldest-first;
/// `dropped` is what the caller should summarize.
///
/// Trimming is turn-aligned: a user message and the assistant reply it produced
/// travel together, so the kept history never opens with an answer to a question
/// that was cut.
pub fn trim_turns(history: Vec<Value>, max_turns: usize, max_tokens: usize) -> (Vec<Value>, Vec<Value>) {
    let starts: Vec<usize> = history
        .iter()
        .enumerate()
        .filter(|(_, m)| starts_turn(m))
        .map(|(i, _)| i)
        .collect();

    // No identifiable turns (an assistant-only or malformed history): fall back to
    // a token-only cut from the front rather than guessing at structure.
    if starts.is_empty() {
        let mut kept = history;
        let mut dropped = Vec::new();
        while estimate_messages(&kept) > max_tokens && !kept.is_empty() {
            dropped.push(kept.remove(0));
        }
        return (kept, dropped);
    }

    // Walk turn starts newest-first, taking turns while both budgets hold. Always
    // keep at least the most recent turn — dropping everything would strand the
    // model with no idea what the conversation is about.
    let mut cut = starts[0];
    let mut turns = 0;
    let mut tokens = 0;
    for (rank, &start) in starts.iter().enumerate().rev() {
        let end = starts.get(rank + 1).copied().unwrap_or(history.len());
        let cost = estimate_messages(&history[start..end]);
        let first = turns == 0;
        if !first && (turns >= max_turns || tokens + cost > max_tokens) {
            break;
        }
        turns += 1;
        tokens += cost;
        cut = start;
    }

    let mut history = history;
    let kept = history.split_off(cut);
    (kept, history)
}

/// Fold dropped turns into a transcript for the summarizer. Long individual
/// messages are clipped: the summary needs the shape of the exchange, not every
/// word of a pasted document.
pub fn transcript(msgs: &[Value]) -> String {
    msgs.iter()
        .filter_map(|m| {
            let role = m["role"].as_str().unwrap_or("?");
            let content = m["content"].as_str()?.trim();
            if content.is_empty() {
                return None;
            }
            Some(format!("{role}: {}", middle_out(content, 2_000)))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Summarize dropped turns so the run keeps their substance at a fraction of the
/// tokens. Runs on the local model by default (this is bookkeeping, not the user's
/// task). On failure the caller still gets a usable — if blunter — note, because
/// losing the summary must never fail the turn.
pub async fn summarize_dropped(settings: &Settings, dropped: &[Value]) -> String {
    use crate::core::model_router::{self, ChatMessage};

    let body = transcript(dropped);
    if body.trim().is_empty() {
        return String::new();
    }
    let msgs = vec![
        ChatMessage {
            role: "system".into(),
            content: "Summarize this earlier part of a conversation for an assistant that will keep working on it. \
                      Keep decisions, facts, file paths, names, numbers and anything the user asked for that is still \
                      outstanding. Drop pleasantries. Write compact prose or bullets, no preamble, under 200 words."
                .into(),
            images: vec![],
            docs: vec![],
        },
        ChatMessage { role: "user".into(), content: middle_out(&body, 12_000), images: vec![], docs: vec![] },
    ];
    match model_router::complete(settings, &msgs, model_router::Workload::Background).await {
        Ok((s, _)) if !s.trim().is_empty() => s.trim().to_string(),
        // No summarizer available (Ollama down, no key): say so plainly rather than
        // implying the earlier turns are gone without trace.
        _ => format!("({} earlier messages were dropped to fit the context window and could not be summarized.)", dropped.len()),
    }
}

/// What `/context` reports.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextStats {
    pub messages: usize,
    pub user: usize,
    pub assistant: usize,
    pub chars: usize,
    pub est_tokens: usize,
    pub max_tokens: usize,
    pub max_turns: usize,
    pub pct: u8,
}

pub fn stats(history: &[Value], budget: Budget) -> ContextStats {
    let est = estimate_messages(history);
    ContextStats {
        messages: history.len(),
        user: history.iter().filter(|m| m["role"] == "user").count(),
        assistant: history.iter().filter(|m| m["role"] == "assistant").count(),
        chars: history.iter().filter_map(|m| m["content"].as_str()).map(|s| s.chars().count()).sum(),
        est_tokens: est,
        max_tokens: budget.max_tokens,
        max_turns: budget.max_turns,
        pct: ((est as f64 / budget.max_tokens.max(1) as f64) * 100.0).round().min(999.0) as u8,
    }
}
