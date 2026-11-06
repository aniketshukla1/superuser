use super::*;

/// At or below this many tools, nothing is narrowed.
pub const FLOOR: usize = 12;

/// How long the chooser gets before the run goes ahead with everything.
pub(super) const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

pub(super) const INSTRUCTIONS: &str = "Decide which of a person's saved procedures (\"skills\") this request will need.\n\n\
Reply with JSON only: {\"skills\": [\"skill-name\", ...]}, using the names exactly as listed.\n\n\
Err on the side of including. An extra skill costs next to nothing; a missing one leaves the work impossible. \
If unsure, include it. If no skill relates to the request, reply {\"skills\": []}.";

/// Why a run offered what it offered — for the trace, and for the ablation's report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Fewer tools than the floor.
    UnderFloor,
    /// No skill declares any tools.
    NothingDeclared,
    /// The chooser could not answer.
    Unavailable,
    /// It answered, and picked none.
    NothingChosen,
    /// It answered and picked some.
    Selected,
}

impl Why {
    pub fn label(self) -> &'static str {
        match self {
            Why::UnderFloor => "under-floor",
            Why::NothingDeclared => "nothing-declared",
            Why::Unavailable => "unavailable",
            Why::NothingChosen => "nothing-chosen",
            Why::Selected => "selected",
        }
    }
}

pub fn on() -> bool {
    std::env::var("SUPERUSER_SKILL_NARROWING").is_ok_and(|v| v == "1" || v == "true")
}

/// The skill names the chooser picked, or `None` when it could not say.
///
/// Reads the LAST user message only: an early mention of a spreadsheet should not keep the
/// spreadsheet tools loaded for the rest of a conversation about something else.
pub async fn chosen(settings: &Settings, skills: &[&Skill], message: &str) -> Option<Vec<String>> {
    let menu: String = skills
        .iter()
        .map(|s| format!("- {}: {}\n", s.name, s.description))
        .collect();
    let messages = vec![
        model_router::ChatMessage { role: "system".into(), content: INSTRUCTIONS.into(), images: vec![], docs: vec![] },
        model_router::ChatMessage {
            role: "user".into(),
            content: format!("The skills:\n{menu}\nThe request:\n{}", message.chars().take(4000).collect::<String>()),
            images: vec![],
            docs: vec![],
        },
    ];
    let call = model_router::complete(settings, &messages, Workload::Background);
    let reply = match tokio::time::timeout(TIMEOUT, call).await {
        Ok(Ok((reply, _))) if !reply.trim().is_empty() => reply,
        _ => return None,
    };
    parse(&reply, skills)
}

/// The names out of a reply, dropping anything that is not a skill on this machine.
///
/// A model that answers with prose around its JSON is answering; a model that invents a
/// skill name is not, and the invention is dropped rather than failing the whole pick.
pub fn parse(reply: &str, skills: &[&Skill]) -> Option<Vec<String>> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    let value: serde_json::Value = serde_json::from_str(reply.get(start..=end)?).ok()?;
    let picked = value["skills"].as_array()?;
    Some(
        picked
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|name| skills.iter().find(|s| s.name.eq_ignore_ascii_case(name.trim())).map(|s| s.name.clone()))
            .collect(),
    )
}

/// What to offer: everything the chosen skills declare, plus every granted tool no skill
/// claims at all.
///
/// The second half is what keeps this safe. A tool nothing has declared is not a tool
/// nobody needs — it is one nobody has written a skill about — and withholding it would
/// make the narrowing's blind spot the model's problem.
pub fn offered(specs: &[ToolSpec], skills: &[&Skill], chosen: &[String]) -> Vec<ToolSpec> {
    let claimed: BTreeSet<&str> = skills.iter().flat_map(|s| s.allowed_tools.iter().map(String::as_str)).collect();
    let wanted: BTreeSet<&str> = skills
        .iter()
        .filter(|s| chosen.iter().any(|c| c == &s.name))
        .flat_map(|s| s.allowed_tools.iter().map(String::as_str))
        .collect();
    specs
        .iter()
        .filter(|t| wanted.contains(t.name.as_str()) || !claimed.contains(t.name.as_str()))
        .cloned()
        .collect()
}

/// Whether there is anything to narrow at all — the two refusals that need no model.
pub fn worth_asking(specs: &[ToolSpec], skills: &[&Skill]) -> Result<(), Why> {
    if specs.len() <= FLOOR {
        return Err(Why::UnderFloor);
    }
    if skills.iter().all(|s| s.allowed_tools.is_empty()) {
        return Err(Why::NothingDeclared);
    }
    Ok(())
}

/// What to offer, given what the chooser said. `None` is "it could not say", and every
/// way of not saying ends the same: offer everything.
///
/// Pure, and separate from the call that produces `picked`, so the fail-open paths can be
/// tested without a model — a test that reaches a model tests the machine it runs on.
pub fn decide(specs: &[ToolSpec], skills: &[&Skill], picked: Option<Vec<String>>) -> (Vec<ToolSpec>, Why) {
    if let Err(why) = worth_asking(specs, skills) {
        return (specs.to_vec(), why);
    }
    match picked {
        None => (specs.to_vec(), Why::Unavailable),
        Some(picked) if picked.is_empty() => (specs.to_vec(), Why::NothingChosen),
        Some(picked) => (offered(specs, skills, &picked), Why::Selected),
    }
}

/// The whole decision: what this run should offer, and why.
pub async fn narrow(settings: &Settings, specs: &[ToolSpec], skills: &[&Skill], message: &str) -> (Vec<ToolSpec>, Why) {
    if let Err(why) = worth_asking(specs, skills) {
        return (specs.to_vec(), why);
    }
    decide(specs, skills, chosen(settings, skills, message).await)
}
