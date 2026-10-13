use super::*;

impl Subconscious {
    pub fn load(data_dir: &Path) -> Self {
        let watches = fs::read_to_string(data_dir.join("watches.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { data_dir: data_dir.to_path_buf(), watches }
    }

    pub(super) fn save(&self) {
        // Atomic: the tick loop writes this while the API is serving the Watches
        // page off it, and a torn read comes back as "you have no watches" —
        // which a load-mutate-save caller would then write back. See `core::store`.
        let _ = crate::core::store::write_json_atomic(&self.data_dir.join("watches.json"), &self.watches);
    }

    pub fn create(&mut self, w: Watch) -> Watch {
        self.watches.push(w.clone());
        self.save();
        w
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.watches.len();
        self.watches.retain(|w| w.id != id);
        let gone = self.watches.len() < before;
        if gone {
            self.save();
        }
        gone
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(w) = self.watches.iter_mut().find(|w| w.id == id) {
            w.enabled = enabled;
            self.save();
            return true;
        }
        false
    }

    pub(super) fn mark_ticked(&mut self, id: &str) {
        if let Some(w) = self.watches.iter_mut().find(|w| w.id == id) {
            w.last_tick = Some(now());
        }
        self.save();
    }

    pub(super) fn activity_path(&self) -> PathBuf {
        self.data_dir.join("subconscious-activity.json")
    }

    pub fn activity(&self) -> Vec<Activity> {
        fs::read_to_string(self.activity_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub(super) fn record(&self, a: Activity) {
        let mut all = self.activity();
        all.insert(0, a);
        all.truncate(ACTIVITY_LIMIT);
        let _ = crate::core::store::write_json_atomic(&self.activity_path(), &all);
    }
}

/// The prompt is deliberately about *triage*, not about doing the work. Asking a
/// small local model for one word plus a reason is something it can do reliably;
/// asking it to also decide how to act is not.
pub(super) const TRIAGE_SYSTEM: &str = "You triage background work for a personal assistant.

You are given something the user asked to be watched, and a list of what has changed
since the last check. Decide how much attention it deserves.

Answer with exactly one word on the first line — skip, act, or escalate — then one
short sentence saying why.

- skip: nothing here is relevant to the watch. This is the normal answer; most
  changes are noise for any given watch.
- act: relevant, and answerable by reading and summarizing what changed.
- escalate: relevant, and needs judgement, several steps, or an action with a real
  effect. Use this sparingly.

Never invent items that are not in the list.";

/// Ask the local model what to do about one watch. Returns the decision and its
/// stated reason.
pub(super) async fn triage(settings: &Settings, watch: &Watch, evidence: &str) -> Result<(Decision, String), String> {
    use crate::core::model_router::{self, ChatMessage};
    let msgs = vec![
        ChatMessage { role: "system".into(), content: TRIAGE_SYSTEM.into(), images: vec![], docs: vec![] },
        ChatMessage {
            role: "user".into(),
            content: format!("## Watch\n\n{}\n\n## What changed\n\n{}", watch.question.trim(), evidence.trim()),
            images: vec![],
            docs: vec![],
        },
    ];
    // `false` = do not prefer cloud. This loop runs on the local model by design;
    // routing it to a paid provider every few minutes is the thing that makes
    // always-on assistants unaffordable.
    let (raw, _) = model_router::complete(settings, &msgs, model_router::Workload::Background).await?;
    let (decision, mut reason) = Decision::read(&raw);
    if reason.is_empty() {
        // A verdict with nothing behind it still has to read as something in the
        // log, and "(no reason given)" is at least true.
        reason = "(no reason given)".into();
    }
    Ok((decision, reason))
}

/// Answer a watch locally, given what changed. Read-only by construction: it has
/// no tools, so the worst it can do is write a note.
pub(super) async fn answer(settings: &Settings, watch: &Watch, evidence: &str) -> Result<String, String> {
    use crate::core::model_router::{self, ChatMessage};
    let msgs = vec![
        ChatMessage {
            role: "system".into(),
            content: "Answer the user's standing question using only the changes listed. \
                      Be specific and brief — a few sentences or bullets. If the changes don't \
                      really answer it, say what they do show and stop. Never invent items."
                .into(),
            images: vec![],
            docs: vec![],
        },
        ChatMessage {
            role: "user".into(),
            content: format!("## Question\n\n{}\n\n## What changed\n\n{}", watch.question.trim(), evidence.trim()),
            images: vec![],
            docs: vec![],
        },
    ];
    let (out, _) = model_router::complete(settings, &msgs, model_router::Workload::Background).await?;
    Ok(out.trim().to_string())
}

/// Gather the delta for a watch without consuming it, plus a one-line description
/// of what was found. Returns `None` when nothing changed anywhere it watches — the
/// common case, and the one that must cost nothing at all.
pub(super) fn peek(ledger: &Ledger, sources: &[String]) -> Option<(String, Vec<Changes>)> {
    let sources: Vec<String> = if sources.is_empty() { vec!["*".into()] } else { sources.to_vec() };
    let mut found = Vec::new();
    let mut lines = Vec::new();
    for s in &sources {
        let Ok(c) = ledger.changes_since_read(s, false) else { continue };
        if c.is_empty() {
            continue;
        }
        lines.push(c.summary());
        found.push(c);
    }
    if found.is_empty() {
        return None;
    }
    Some((lines.join("\n"), found))
}

/// Render a delta as the material the model reads. Titles only: the decision is
/// "does this matter", which titles answer, and bodies would blow a small model's
/// window on a busy morning.
pub(super) fn evidence_text(memory: &crate::core::memory::MemoryEngine, changed: &[Changes]) -> String {
    let mut out = String::new();
    for c in changed {
        out.push_str(&format!("\n### {}\n", c.summary()));
        for f in c.added.iter().chain(c.modified.iter()).take(25) {
            let title = memory
                .parse_file(&memory.vault_path.join(f))
                .map(|m| m.title)
                .unwrap_or_else(|| f.clone());
            out.push_str(&format!("- {title}\n"));
        }
    }
    out
}
