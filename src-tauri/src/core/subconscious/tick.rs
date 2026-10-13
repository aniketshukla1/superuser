use super::*;

/// Hand a watch to the user rather than deciding alone: park an ask in the Inbox.
///
/// This is deliberately the *only* thing escalation does. The tempting version runs
/// the full agent with tools — but this loop's input is, by construction, content
/// from outside the machine, and a background process that acts on it unprompted is
/// the exfiltration path taint was added to close. Noticing is the loop's job;
/// deciding is the user's.
pub(super) fn escalate(data_dir: &Path, w: &Watch, headline: &str, reason: &str) -> String {
    let mut inbox = crate::core::inbox::Inbox::load(data_dir);
    let ask = inbox.park(crate::core::inbox::Ask::new(
        // A synthetic session id, so the Inbox card can be traced back to the watch
        // that raised it rather than looking like it came from a conversation.
        &format!("__watch__{}", w.id),
        None,
        "subconscious_escalation",
        &w.sources.join(","),
        &format!("{} — {}", w.question.trim(), reason),
        serde_json::json!({ "watch": w.question, "evidence": headline, "reason": reason }),
    ));
    ask.id
}

/// What one tick did, for the caller (a scheduler) to log.
#[derive(Debug, Clone, PartialEq)]
pub struct TickReport {
    pub evaluated: usize,
    pub skipped_quiet: usize,
    pub acted: usize,
    pub escalated: usize,
    pub failed: usize,
}

/// Run one pass over every enabled watch.
///
/// Independent per watch: one that fails leaves its own delta unconsumed and does
/// not stop the others. Ticks are not re-entrant — the caller is expected to run
/// exactly one at a time.
pub async fn tick(
    settings: &Settings,
    data_dir: &Path,
    memory: &crate::core::memory::MemoryEngine,
) -> TickReport {
    let mut store = Subconscious::load(data_dir);
    let ledger = Ledger::new(&memory.vault_path);
    let mut report = TickReport { evaluated: 0, skipped_quiet: 0, acted: 0, escalated: 0, failed: 0 };

    let watches: Vec<Watch> = store.watches.iter().filter(|w| w.enabled).cloned().collect();
    for w in watches {
        // Nothing changed → no model call, no activity line. A quiet loop should be
        // quiet in the log too, or the record of what it did becomes unreadable.
        let Some((headline, changed)) = peek(&ledger, &w.sources) else {
            report.skipped_quiet += 1;
            continue;
        };
        report.evaluated += 1;
        let evidence = evidence_text(memory, &changed);

        let (decision, reason) = match triage(settings, &w, &evidence).await {
            Ok(d) => d,
            Err(e) => {
                // The marker stays put: these items have not been looked at.
                report.failed += 1;
                store.record(Activity {
                    id: new_id("act"),
                    watch_id: w.id.clone(),
                    question: w.question.clone(),
                    at: now(),
                    decision: Decision::Skip,
                    reason: format!("couldn't evaluate: {e}"),
                    evidence: headline,
                    note: None,
                    ask_id: None,
                });
                continue;
            }
        };

        let mut note = None;
        let mut ask_id = None;
        match decision {
            Decision::Skip => {}
            Decision::Act => {
                match answer(settings, &w, &evidence).await {
                    Ok(text) => {
                        note = Some(text);
                        report.acted += 1;
                    }
                    Err(e) => {
                        report.failed += 1;
                        note = Some(format!("(couldn't answer: {e})"));
                    }
                }
            }
            Decision::Escalate => {
                ask_id = Some(escalate(data_dir, &w, &headline, &reason));
                report.escalated += 1;
            }
        }

        store.record(Activity {
            id: new_id("act"),
            watch_id: w.id.clone(),
            question: w.question.clone(),
            at: now(),
            decision,
            reason,
            evidence: headline,
            note,
            ask_id,
        });

        // Looked at, and something was done about it (including deciding to do
        // nothing). Only now is it honest to say these items have been seen.
        for c in &changed {
            let _ = ledger.acknowledge(&c.source);
        }
        store.mark_ticked(&w.id);
    }
    report
}
