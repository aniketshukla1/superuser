use super::super::*;

/// Distill recent daily memory into the core memory, and write a diary of what
/// changed. Safe to call at any time: it skips when there is no new material, and
/// leaves the core file untouched unless it got a well-formed replacement.
///
/// Runs on the local model by default — this is housekeeping, not the user's task.
pub async fn deep_dream(
    settings: &crate::core::settings::Settings,
    journal: &Journal,
    lookback_days: usize,
) -> DreamOutcome {
    use crate::core::model_router::{self, ChatMessage};

    let lookback = lookback_days.clamp(1, 30);
    let dailies = journal.recent_dailies(lookback);
    if dailies.iter().all(|d| d.body.trim().is_empty()) {
        return DreamOutcome::Skipped("no daily memory to consolidate");
    }

    let fingerprint = journal.input_fingerprint(lookback);
    if journal.last_fingerprint() == Some(fingerprint) {
        return DreamOutcome::Skipped("nothing has changed since the last pass");
    }

    let core = journal.read_core();
    // Evidence only. The recaps for these same days exist and are cheaper to read,
    // which is exactly why they are not used here: distilling an interpretation
    // would make the core memory a summary of summaries with nothing to check it
    // against. See the module docs.
    let material = dailies
        .iter()
        .rev() // oldest first, so "newer wins" reads in the natural direction
        .map(|d| format!("### {}\n{}", d.date, d.body.trim()))
        .collect::<Vec<_>>()
        .join("\n\n");

    let user = format!(
        "## Current MEMORY.md\n\n{}\n\n## Recent daily memory\n\n{}",
        if core.trim().is_empty() { "(empty — this is the first consolidation)" } else { core.trim() },
        crate::core::context::middle_out(&material, 24_000),
    );

    let msgs = vec![
        ChatMessage {
            role: "system".into(),
            content: DREAM_SYSTEM.replace("{CORE_TARGET_ENTRIES}", &CORE_TARGET_ENTRIES.to_string()),
            images: vec![],
            docs: vec![],
        },
        ChatMessage { role: "user".into(), content: user, images: vec![], docs: vec![] },
    ];
    let raw = match model_router::complete(settings, &msgs, model_router::Workload::BackgroundCapable).await {
        Ok((s, _)) => s,
        Err(e) => return DreamOutcome::Failed(e),
    };

    let Some((new_core, diary)) = parse_dream(&raw) else {
        return DreamOutcome::Failed("the model's answer wasn't in a form safe to write".into());
    };

    // A citation to a day the model was never shown is worse than no citation: it
    // reads as evidence and is not. The bullets stay either way — a fact that turns
    // out to be uncited is still a fact — but the provenance has to be real, or the
    // link back to the evidence this whole three-tier design exists for is decorative.
    let days: Vec<String> = dailies.iter().map(|d| d.date.clone()).collect();
    let (new_core, invented) = verify_citations(&new_core, &days);

    if journal.write_core(&new_core).is_err() {
        return DreamOutcome::Failed("could not write MEMORY.md".into());
    }
    let entries = new_core.lines().filter(|l| l.trim_start().starts_with('-')).count();
    let mut diary = if diary.is_empty() {
        format!("Consolidated {} days of daily memory into {entries} core entries.", dailies.len())
    } else {
        diary
    };
    // Reported, not swallowed. A pass that invents sources is a pass worth looking
    // at, and the diary is the place someone would look.
    if invented > 0 {
        diary.push_str(&format!(
            "\n\n({invented} citation(s) named a day that was not in the material, and were removed.)"
        ));
    }
    let _ = journal.write_dream(&today(), &diary);
    // Only recorded on success: a failed pass must be retried, not skipped as "seen".
    journal.record_fingerprint(fingerprint);

    DreamOutcome::Consolidated { entries, diary }
}
