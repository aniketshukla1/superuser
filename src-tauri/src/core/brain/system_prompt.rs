use super::*;

/// The system prompt.
///
/// Split out from `ask` so it can be asserted on: the instructions here are the
/// whole difference between a memory assistant and a plausible-sounding one, and a
/// prompt that only exists inside an async function that calls a model is a prompt
/// no test can see.
pub fn system_prompt(r: &Recall) -> String {
    let mut s = String::from(
        "You are the user's own brain, answering a question about what you hold on their behalf. \
         Everything below came out of their private vault on this machine — their connected \
         accounts, their notes, and the record of past conversations with them.\n\n\
         How to answer:\n\
         - Lead with the direct answer in the first sentence. Never open by restating the question.\n\
         - Then give the detail that supports it: names, dates, numbers, decisions, and who or what \
         they involved. Be specific — a vague answer from a full vault is a failure.\n\
         - Organise anything longer than a few sentences with short markdown headings or bullets. \
         Group by theme, not by which store the material came from; the user does not care which \
         drawer it was in.\n\
         - For counting, \"how many\", \"which categories\" or date-range questions, use the census \
         below. It is exact. The excerpts are a relevant sample, not the whole vault, so never count \
         from them.\n\
         - To list or enumerate — \"which ones\", \"name all\", \"do I have an X\" — use an INVENTORY \
         block if one is present. It is the complete set for its category, unlike the excerpts. \
         Answering \"I have no record of that\" from the excerpts alone, when an inventory is sitting \
         right there, is the worst answer available: it is wrong, and it sounds certain.\n\
         - Where sources disagree, say so and prefer the more recent one, naming both dates.\n\
         - If the vault does not answer the question, say that plainly and say what it does hold \
         that is nearby. Never invent a memory, a date, a name or a number. An honest gap is useful; \
         a confident fabrication destroys the point of having a memory at all.\n\
         - Do not list your sources at the end — that is added for you, from the real records.\n\n",
    );
    s.push_str(&r.census.block());
    // Next to the census, because it is the same kind of material: exact, complete,
    // and safe to answer from directly — as opposed to the excerpts below, which are
    // a sample and are labelled as one.
    s.push_str(&r.inventory_block());
    if r.is_empty() {
        s.push_str(
            "\nNothing in the vault matched this question. Say so, and use the census above to \
             tell the user what is in there instead.\n",
        );
    }
    s.push_str(&r.block());
    if !r.profile.is_empty() {
        // The profile block is written as instructions about the user ("prefers
        // concise answers"), so it lands last, next to the instructions it modifies.
        s.push_str(&r.profile);
    }
    s
}

/// One `@brain` answer: the model's prose, with our citation footer appended.
pub struct Answer {
    pub text: String,
    /// Which model actually answered, as `model_router` reports it.
    pub provider: String,
}

/// Ask the brain.
///
/// `model` is the model the composer had selected for this turn. A cloud pick
/// raises the budget and gets the better answer — but it can never override
/// `local_only`, which stays exactly where the user set it and is enforced by
/// `model_router::route`. (`commands::memory_ask` clears that flag when handed a
/// cloud model; deliberately not copied. A hard egress switch that any model picker
/// can turn off is not a switch.)
/// Which model a question will run on, and what it is allowed to gather.
///
/// Decided before any work starts, and handed back rather than kept private,
/// because the caller wants to *say* it: "Asking gpt-5.6-luna" is the only useful
/// thing there is to show during the minutes the model is thinking, and a caller
/// that had to wait for the answer to learn the model could only show it afterwards.
pub struct Plan {
    pub settings: Settings,
    /// The model id that will actually be called, after `local_only` has had its say.
    pub model: String,
    pub budget: Budget,
}

/// Resolve the model and the budget. Cheap and synchronous — no vault, no network.
pub fn plan(settings: &Settings, model: Option<&str>) -> Plan {
    use crate::core::providers;

    let mut settings = settings.clone();
    // The model this question runs on: what the composer sent, or the one the
    // settings already resolve to. `effective_model` rather than a local reimagining
    // of the same decision — its doc records what happened the last two times that
    // rule was written out a second time.
    let picked = model.map(str::trim).filter(|m| !m.is_empty()).map(String::from);
    let chosen = picked.clone().unwrap_or_else(|| providers::effective_model(&settings));

    // Local or not is a question about the model's own provider — not about whether
    // an Anthropic key happens to exist. Asking the latter is the bug
    // `providers::cloud_model_ready` was written to kill, and this asked it: on a
    // machine with an Anthropic key, picking Codex or Groq claimed the cloud budget
    // and then sent the prompt to the wrong vendor.
    let local = providers::runs_locally(&chosen) || !providers::cloud_model_ready(&chosen);
    // `local_only` still has the last word, and it is the user's, not a provider's.
    let chosen = if settings.local_only && !providers::runs_locally(&chosen) {
        settings.ollama_model.clone()
    } else {
        chosen
    };
    let local = local || settings.local_only;

    if providers::runs_locally(&chosen) {
        settings.ollama_model = chosen.clone();
    } else {
        settings.cloud_model = chosen.clone();
        settings.cloud_enabled = true;
    }
    let budget = Budget::resolve(local);
    settings.max_tokens = settings.max_tokens.max(budget.answer_tokens);
    Plan { settings, model: chosen, budget }
}

/// Put the question, and what was retrieved for it, to the model.
///
/// Separate from `recall` so a caller with a progress panel can say what it is
/// doing between the two — they are the slow half and the very slow half, and a
/// single call that does both can only report when it is over.
pub async fn answer(plan: &Plan, r: &Recall, query: &str) -> Result<Answer, String> {
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: system_prompt(r),
            images: vec![],
            docs: vec![],
        },
        ChatMessage {
            role: "user".into(),
            content: query.to_string(),
            images: vec![],
            docs: vec![],
        },
    ];
    // `complete_on`, not `complete`: this asks for ONE named model through its own
    // provider. `complete` routes by workload and only knows Anthropic and Ollama,
    // which is how a Codex or Groq pick reached api.anthropic.com.
    let (content, provider) = model_router::complete_on(&plan.settings, &messages, &plan.model).await?;

    Ok(Answer { text: format!("{}{}", content.trim(), r.sources_block()), provider })
}

/// Plan, retrieve and answer in one call — for callers with nothing to report to.
pub async fn ask(
    engine: &MemoryEngine,
    settings: &Settings,
    vault: &Path,
    data_dir: &Path,
    query: &str,
    model: Option<&str>,
) -> Result<Answer, String> {
    let p = plan(settings, model);
    let r = recall(engine, &p.settings, vault, data_dir, query, p.budget).await;
    answer(&p, &r, query).await
}

/// Cut to `max` characters on a character boundary, marking the cut.
///
/// Byte slicing would panic on any vault holding a non-ASCII character, which is to
/// say on most of them.
pub(super) fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{cut}…")
}
