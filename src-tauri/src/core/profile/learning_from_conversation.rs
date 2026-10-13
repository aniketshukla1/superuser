//! Learning from a conversation

use super::*;

/// Asking a small local model for a table is the one shape it does reliably. The
/// prompt is about *noticing*, not judging: deciding what matters is the scorer's
/// job, and it can only do it if the model reports rather than filters.
pub(super) const OBSERVE_SYSTEM: &str = "You extract lasting preferences from things a person said to their assistant.

You are given only the person's own messages. Report anything that says how they want to be
worked with in future conversations. Ignore the task at hand — what they asked for today is
not a preference.

Answer with one line per preference and nothing else, in this format:

class | key | value | cue

- class is one of: identity, style, tooling, veto, goal, channel
- key is 1-3 lowercase words naming the slot, e.g. `call-me`, `reply-length`, `test-runner`
- value is a short instruction in the third person, e.g. `Goes by Nik`, `Wants short answers`
- cue is `explicit` if they said it outright, `behavioral` if you are inferring it

Rules:
- At most 6 lines. Fewer is normal; most messages contain none at all.
- If there is nothing lasting, answer with the single word NONE.
- Never invent a preference to fill the table. A guess costs more than a miss here,
  because this steers every future answer.";

/// What one pass over a conversation learned.
#[derive(Debug, Clone, Default)]
pub struct Learnings {
    pub recorded: usize,
    pub proposed: usize,
}

/// Read a conversation's **user turns** and record what they say about how the
/// user wants to be worked with.
///
/// The argument is user-authored text and only user-authored text. Tool results,
/// synced memories and anything a connector produced must never reach here: the
/// profile is injected into every future system prompt, so a facet learned from
/// third-party content would be a standing instruction written by whoever wrote
/// that content. This is the same boundary Step 1 drew for the run gate, at the
/// one other place content can become durable behaviour.
pub async fn observe_turns(
    settings: &crate::core::settings::Settings,
    data_dir: &Path,
    session_id: &str,
    user_turns: &[String],
) -> Result<Learnings, String> {
    use crate::core::model_router::{self, ChatMessage};

    let corpus: String = user_turns
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| format!("- {}", t.chars().take(1_200).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n");
    if corpus.trim().is_empty() {
        return Ok(Learnings::default());
    }

    let msgs = vec![
        ChatMessage { role: "system".into(), content: OBSERVE_SYSTEM.into(), images: vec![], docs: vec![] },
        ChatMessage {
            role: "user".into(),
            content: format!("What they said:\n\n{}", corpus.chars().take(8_000).collect::<String>()),
            images: vec![],
            docs: vec![],
        },
    ];
    let (raw, _) = model_router::complete(settings, &msgs, model_router::Workload::Background).await?;

    let obs = parse_observations(&raw, session_id, now());
    let proposed = obs.len();
    let mut learned = Learned::load(data_dir);
    let recorded = learned.observe(obs);
    Ok(Learnings { recorded, proposed })
}

/// Parse the model's table. Anything unreadable is dropped rather than guessed at,
/// on the same principle as the watch triage: a background pass that runs
/// unattended must treat an answer it can't read as nothing to do.
pub fn parse_observations(raw: &str, source: &str, at: f64) -> Vec<Observation> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim().trim_start_matches(['-', '*', '#', ' ']).trim();
        if line.is_empty() || line.eq_ignore_ascii_case("none") {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        if parts.len() < 3 {
            continue;
        }
        let Some(class) = Class::parse(parts[0]) else { continue };
        // A model that echoes the format line back at us must not become a facet.
        if parts[1].eq_ignore_ascii_case("key") || parts[2].eq_ignore_ascii_case("value") {
            continue;
        }
        let cue = parts.get(3).and_then(|c| Cue::parse(c)).unwrap_or(Cue::Behavioral);
        let key = clean_key(parts[1]);
        let value = clean_value(parts[2]);
        if key.is_empty() || value.is_empty() {
            continue;
        }
        out.push(Observation {
            class,
            key,
            value,
            cue,
            at,
            evidence: clean_evidence(line),
            source: source.to_string(),
        });
        if out.len() == 6 {
            break;
        }
    }
    out
}
