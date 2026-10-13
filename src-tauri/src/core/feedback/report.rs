use super::*;

pub fn report(entries: &[Entry]) -> Report {
    use std::collections::BTreeMap;
    let mut r = Report::default();
    let mut reasons: BTreeMap<&str, (Reason, usize)> = BTreeMap::new();
    let mut models: BTreeMap<String, usize> = BTreeMap::new();
    let mut tools: BTreeMap<String, usize> = BTreeMap::new();

    // Only the latest verdict per message counts, or changing your mind votes twice.
    let mut latest: BTreeMap<(String, usize), &Entry> = BTreeMap::new();
    for e in entries {
        latest.insert((e.session.clone(), e.message), e);
    }

    for e in latest.values() {
        match e.verdict {
            Verdict::Up => r.up += 1,
            Verdict::Down => {
                r.down += 1;
                if let Some(reason) = e.reason {
                    reasons.entry(reason.as_str()).or_insert((reason, 0)).1 += 1;
                }
                if !e.evidence.model.is_empty() {
                    *models.entry(e.evidence.model.clone()).or_default() += 1;
                }
                for t in &e.evidence.tools {
                    *tools.entry(t.clone()).or_default() += 1;
                }
            }
        }
    }

    let sort = |mut v: Vec<(String, usize)>| {
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    };
    r.by_reason = {
        let mut v: Vec<(Reason, usize)> = reasons.into_values().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.as_str().cmp(b.0.as_str())));
        v
    };
    r.by_model = sort(models.into_iter().collect());
    r.tools_on_downvotes = sort(tools.into_iter().collect());
    r
}

impl Report {
    /// The report as markdown, for a person deciding what to work on.
    pub fn render(&self) -> String {
        if self.up + self.down == 0 {
            return "No feedback recorded yet.\n".into();
        }
        let mut s = format!("## Feedback\n\n{} up, {} down\n", self.up, self.down);
        if !self.by_reason.is_empty() {
            s.push_str("\n| what was wrong | n | look at |\n|---|---|---|\n");
            for (reason, n) in &self.by_reason {
                s.push_str(&format!("| {} | {n} | {} |\n", reason.label(), reason.points_at()));
            }
        }
        if self.by_model.len() > 1 {
            s.push_str("\n**By model** — a harness problem shows up on every model; a model problem does not.\n\n");
            for (m, n) in &self.by_model {
                s.push_str(&format!("* `{m}` — {n}\n"));
            }
        }
        if !self.tools_on_downvotes.is_empty() {
            s.push_str(
                "\n**Tools present on downvoted turns.** Presence, not blame — compare each \
                 against its share in `toolstats` before reading anything into it.\n\n",
            );
            for (t, n) in self.tools_on_downvotes.iter().take(10) {
                s.push_str(&format!("* `{t}` — {n}\n"));
            }
        }
        s
    }
}

/// A fresh entry with the clock already read.
pub fn entry(session: &str, message: usize, verdict: Verdict) -> Entry {
    Entry {
        at: now(),
        session: session.to_string(),
        message,
        message_at: 0.0,
        verdict,
        reason: None,
        note: String::new(),
        evidence: Evidence::default(),
    }
}
