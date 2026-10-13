use super::*;

impl MemoryEngine {
    /// Which of these memories a newer one with the same title has overtaken.
    ///
    /// Returns, per filename, the date of the newest memory sharing its title — or
    /// `None` when this one *is* the newest. The comparison is over the `date`
    /// frontmatter and nothing else: the model is never asked which of two facts
    /// looks more current, because asked that question models confabulate and apply
    /// inconsistent criteria (*Don't Ask the LLM to Track Freshness*, 2606.01435).
    /// Extraction is the model's job; policy is ours, and this is policy.
    ///
    /// Nothing is hidden. The caller states the fact and orders by it; both entries
    /// stay readable, because "same title" is evidence about recency and not proof
    /// of contradiction.
    pub fn freshness(hits: &[Memory]) -> std::collections::HashMap<String, Option<String>> {
        let mut newest: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
        for m in hits {
            let k = m.title.trim().to_lowercase();
            let e = newest.entry(k).or_insert(&m.date);
            if m.date.as_str() > *e {
                *e = &m.date;
            }
        }
        hits.iter()
            .map(|m| {
                let top = newest.get(&m.title.trim().to_lowercase()).copied().unwrap_or("");
                let overtaken = (top > m.date.as_str()).then(|| top.to_string());
                (m.filename.clone(), overtaken)
            })
            .collect()
    }

    /// Every memory that matched at all, scored and unsorted. The shared half of
    /// the two recalls above, so they cannot drift apart.
    pub(in crate::core::memory) fn score_all(&self, query: &str) -> Vec<(Memory, f64)> {
        self.score_over(self.recallable(), query)
    }

    /// The scoring itself, over whatever set the caller says is in play.
    pub(in crate::core::memory) fn score_over(&self, memories: Vec<Memory>, query: &str) -> Vec<(Memory, f64)> {
        let tokens: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|t| t.len() > 2)
            .map(|t| t.to_string())
            .collect();
        if tokens.is_empty() {
            return vec![];
        }

        memories
            .into_iter()
            .map(|m| {
                let title = m.title.to_lowercase();
                let body = m.body.to_lowercase();
                let summary = m.summary.to_lowercase();
                let mut score = 0.0;
                for t in &tokens {
                    if m.tags.iter().any(|tag| tag.to_lowercase() == *t) {
                        score += 15.0;
                    }
                    if title.contains(t) {
                        score += 8.0;
                    }
                    if summary.contains(t) {
                        score += 5.0;
                    }
                    let freq = body.matches(t.as_str()).count();
                    if freq > 0 {
                        score += (freq as f64 * 1.5).min(10.0);
                    }
                }
                let trust = if m.origin == Origin::Internal { INTERNAL_TRUST_BOOST } else { 1.0 };
                let weighted = score * m.weight * trust;
                (m, weighted)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect()
    }

    /// Reinforce a memory on use: bump weight + count, reset last_interaction.
    ///
    /// Carries `origin` through unchanged. Being read often is not evidence that
    /// content is trustworthy — if anything the reverse — so a rewrite here must
    /// never be able to launder an external memory into an internal one.
    pub fn reinforce(&self, filename: &str) -> Option<Memory> {
        let path = self.vault_path.join(filename);
        let m = self.parse_file(&path)?;
        let new_base = (m.weight + REINFORCE_BONUS).min(1.0);
        let input = MemoryInput {
            filename: Some(filename.to_string()),
            title: m.title,
            category: m.category,
            tags: m.tags,
            summary: m.summary,
            body: m.body,
            origin: m.origin,
            key: None,
            supersedes: None,
        };
        // Re-write with bumped metadata.
        let now = Utc::now().to_rfc3339();
        let tags = format!("[{}]", input.tags.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(", "));
        let content = format!(
            "---\ntitle: \"{}\"\ncategory: \"{}\"\ntags: {}\ndate: \"{}\"\nsummary: \"{}\"\nweight: {:.3}\nreinforce_count: {}\nlast_interaction: \"{}\"\norigin: \"{}\"\nkey: \"{}\"\nsupersedes: \"{}\"\nsuspect: {}\n---\n\n{}",
            esc(&input.title), esc(&input.category), tags, m.date, esc(&input.summary),
            new_base, m.reinforce_count + 1, now, input.origin.as_str(), esc(&m.key), esc(&m.supersedes), m.suspect, input.body
        );
        fs::write(&path, content).ok()?;
        self.parse_file(&path)
    }
}
