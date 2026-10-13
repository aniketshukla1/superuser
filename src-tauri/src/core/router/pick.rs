use super::*;

/// How sure the model has to be before its answer is used.
pub const FLOOR: f32 = 0.6;

pub(super) const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

pub(super) const INSTRUCTIONS: &str = "You route a message to one of a person's assistants.\n\n\
Answer with JSON and nothing else: {\"assistant\": \"<id>\", \"confidence\": 0.0-1.0}. \
Use an id exactly as given. Confidence is how sure you are that this assistant, rather than \
another, is the right one for this message. If it could be any of them, say so with a low number.";

/// Why this assistant, in words that can go in the trail and in front of a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Only one assistant to choose from.
    OnlyOne,
    /// The only one that can reach a system the message names.
    System,
    /// The chooser picked it.
    Model,
    /// The chooser picked, but was not sure enough.
    LowConfidence,
    /// The chooser could not be reached.
    Unavailable,
    /// The chooser answered something that was not an assistant.
    NoAnswer,
}

impl Why {
    pub fn label(self) -> &'static str {
        match self {
            Why::OnlyOne => "the only assistant",
            Why::System => "the only one that can reach what the message names",
            Why::Model => "chosen for this message",
            Why::LowConfidence => "nothing stood out, so the default",
            Why::Unavailable => "the chooser wasn't available, so the default",
            Why::NoAnswer => "the chooser didn't name an assistant, so the default",
        }
    }

    /// Whether this was a real choice or a fallback. The fallbacks are the ones worth
    /// counting: a router that falls back every time is a router nobody should trust.
    pub fn chose(self) -> bool {
        matches!(self, Why::OnlyOne | Why::System | Why::Model)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub persona: String,
    pub why: Why,
    pub confidence: f32,
}

/// The assistants a message can be routed to: switched on, and visible to the person.
pub fn candidates(reg: &PersonaRegistry) -> Vec<PersonaListItem> {
    reg.list_all().into_iter().filter(|p| p.enabled).collect()
}

/// The only candidate that can reach a system this message names, if there is exactly one.
///
/// "Exactly one" is the whole rule. Two assistants holding Slack is not a routing decision
/// this can make, and guessing between them would be worse than asking the model.
pub fn by_system(candidates: &[PersonaListItem], message: &str) -> Option<String> {
    let said = message.to_lowercase();
    let mut hit: Option<String> = None;
    for p in candidates {
        let systems: Vec<String> = p
            .recommends
            .iter()
            .filter(|r| r.kind == "connector")
            .map(|r| r.reference.to_lowercase())
            .collect();
        // Word-ish match: `linear` must not fire on "linearly".
        let names_one = systems.iter().any(|s| {
            said.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == s)
        });
        if names_one {
            if hit.is_some() {
                return None; // two desks hold it; this is not a decision to make here
            }
            hit = Some(p.id.clone());
        }
    }
    hit
}

/// The id and confidence out of a reply, or `None` when it named nobody real.
pub fn parse(reply: &str, candidates: &[PersonaListItem]) -> Option<(String, f32)> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    let v: serde_json::Value = serde_json::from_str(reply.get(start..=end)?).ok()?;
    let named = v["assistant"].as_str()?.trim().to_lowercase();
    let id = candidates.iter().find(|p| p.id.to_lowercase() == named || p.name.to_lowercase() == named)?.id.clone();
    let confidence = v["confidence"].as_f64().unwrap_or(0.0) as f32;
    Some((id, confidence.clamp(0.0, 1.0)))
}

/// Where this message goes. Never fails: every unknown ends at `default_id` with a reason.
pub async fn pick(settings: &Settings, reg: &PersonaRegistry, message: &str, default_id: &str) -> Choice {
    let cands = candidates(reg);
    let fallback = |why: Why| Choice { persona: default_id.to_string(), why, confidence: 0.0 };
    match cands.len() {
        0 => return fallback(Why::OnlyOne),
        1 => return Choice { persona: cands[0].id.clone(), why: Why::OnlyOne, confidence: 1.0 },
        _ => {}
    }
    if let Some(id) = by_system(&cands, message) {
        return Choice { persona: id, why: Why::System, confidence: 1.0 };
    }
    let menu: String = cands.iter().map(|p| format!("- {}: {} — {}\n", p.id, p.name, p.tagline)).collect();
    let messages = vec![
        ChatMessage { role: "system".into(), content: INSTRUCTIONS.into(), images: vec![], docs: vec![] },
        ChatMessage {
            role: "user".into(),
            content: format!("The assistants:\n{menu}\nThe message:\n{}", message.chars().take(2000).collect::<String>()),
            images: vec![],
            docs: vec![],
        },
    ];
    let call = model_router::complete(settings, &messages, Workload::Background);
    let reply = match tokio::time::timeout(TIMEOUT, call).await {
        Ok(Ok((reply, _))) if !reply.trim().is_empty() => reply,
        _ => return fallback(Why::Unavailable),
    };
    match parse(&reply, &cands) {
        None => fallback(Why::NoAnswer),
        Some((_, c)) if c < FLOOR => fallback(Why::LowConfidence),
        Some((id, c)) => Choice { persona: id, why: Why::Model, confidence: c },
    }
}

/// Write down where a message went and why. Every routing decision, including the ones
/// that were really fallbacks — the fallbacks are the ones worth being able to count.
pub fn record(data_dir: &std::path::Path, session: &str, choice: &Choice) {
    crate::core::audit::log(
        data_dir,
        "route",
        session,
        &choice.persona,
        &format!("{} ({:.0}% sure)", choice.why.label(), choice.confidence * 100.0),
        true,
    );
}
