use super::*;

impl MemoryEngine {
    /// Keyword-relevance recall, amplified by each memory's current weight so
    /// hot, reinforced memories surface first. (Vector recall is a follow-up.)
    pub fn recall(&self, query: &str, limit: usize) -> Vec<Citation> {
        self.recall_scored(query, limit)
            .into_iter()
            .map(|(m, s)| Citation {
                filename: m.filename,
                title: m.title,
                category: m.category,
                score: s.round() as i64,
            })
            .collect()
    }

    /// The same recall, keeping the memories it loaded.
    ///
    /// [`recall`](Self::recall) reduces them to citations, so every caller that
    /// wanted the actual content called `list()` afterwards to get it back —
    /// reading and parsing every file in the vault a second time. Measured on a
    /// 5,000-memory vault: 125 ms for the recall, 115 ms to read it all again. That
    /// second read was free when the only caller was a tool the model had chosen to
    /// use; it stopped being free when SuperContext started sweeping on turn 1 of
    /// every conversation, where it is latency before the first token.
    pub fn recall_scored(&self, query: &str, limit: usize) -> Vec<(Memory, f64)> {
        let mut scored = self.score_all(query);
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// The same scoring, ranked **within each origin** rather than across both.
    ///
    /// One pool is the wrong shape whenever the two populations are read for
    /// different purposes. SuperContext inlines internal memories and only counts
    /// external ones, so a shared top-N silently starves the tier that carries the
    /// content: internal memories are a small minority of any synced vault, and as
    /// the vault grows they stop clearing a bar they are not really competing for.
    ///
    /// Measured before this existed, on the fixture vault in `scale.rs` (5% internal,
    /// which is *generous* — the author's real vault was 125 memories and 0 internal):
    /// the sweep inlined 4 memories at 200, **0 at 1,000 and 0 at 5,000**. The scale
    /// test printed that zero on every run and asserted only on wall-clock.
    ///
    /// Still one `list()`: the vault is read and parsed once and the scored hits are
    /// partitioned in memory, so this costs the same as the single-pool call it
    /// replaces. That matters — this runs on turn 1 of every conversation.
    /// Suspect memories are *counted* here, and only here.
    ///
    /// Everywhere else a suspect memory is out of recall, because recall puts
    /// content in front of the model. The external half of this split never does:
    /// it yields a source name, a count and a date range, all of them produced by
    /// us. So counting a screened memory carries none of the risk screening exists
    /// for — and not counting it would mean the user is not told the thing is in
    /// their vault, which is the wrong kind of quiet.
    pub fn recall_scored_split(
        &self,
        query: &str,
        internal_limit: usize,
        external_limit: usize,
    ) -> (Vec<(Memory, f64)>, Vec<(Memory, f64)>) {
        let all = self.list();
        let dead = Self::superseded(&all);
        let visible: Vec<Memory> = all.into_iter().filter(|m| !dead.contains(&m.filename)).collect();
        let (mut internal, mut external): (Vec<_>, Vec<_>) = self
            .score_over(visible, query)
            .into_iter()
            // Belt and braces: `screen` never marks internal content, and if that
            // ever changed the user's own words must still not be inlined blind.
            .filter(|(m, _)| !(m.origin == Origin::Internal && m.suspect))
            .partition(|(m, _)| m.origin == Origin::Internal);
        for pool in [&mut internal, &mut external] {
            pool.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }
        internal.truncate(internal_limit);
        external.truncate(external_limit);
        (internal, external)
    }

    /// Collapse memories whose title and body are byte-identical.
    ///
    /// Returns the filenames removed. The survivor is the *oldest* — it is the one
    /// other things may already reference — and it inherits the highest reinforce
    /// count in the group, so collapsing copies never costs the user the record of
    /// having used one of them.
    ///
    /// Why this is worth doing at all, beyond tidiness: a duplicate is not neutral
    /// in a similarity ranking, it is a second chance to be retrieved. Unbounded,
    /// redundant memory banks measure *higher* mean similarity to a query and
    /// *lower* precision, because near-copies crowd the top-k with one fact
    /// (2606.29178: 20.2% → 12.4% precision@5 under noise). Deduplication is the
    /// cheapest half of the fix and the half that cannot be wrong.
    pub fn dedup(&self) -> Vec<String> {
        use std::collections::HashMap;
        let all = self.list();
        let mut groups: HashMap<(String, String), Vec<Memory>> = HashMap::new();
        for m in all {
            groups.entry((m.title.clone(), m.body.clone())).or_default().push(m);
        }
        let mut removed = vec![];
        for (_, mut group) in groups {
            if group.len() < 2 {
                continue;
            }
            group.sort_by(|a, b| a.date.cmp(&b.date));
            let best = group.iter().map(|m| m.reinforce_count).max().unwrap_or(0);
            let keep = group.remove(0);
            for m in group {
                if self.delete(&m.filename) {
                    removed.push(m.filename);
                }
            }
            // Re-stamp the survivor only when a copy had earned more than it did.
            if best > keep.reinforce_count {
                let path = self.vault_path.join(&keep.filename);
                if let Ok(text) = fs::read_to_string(&path) {
                    let patched = text.replacen(
                        &format!("reinforce_count: {}", keep.reinforce_count),
                        &format!("reinforce_count: {best}"),
                        1,
                    );
                    let _ = fs::write(&path, patched);
                }
            }
        }
        removed.sort();
        removed
    }

    /// Move synced memories nobody has touched out of recall's way.
    ///
    /// Returns the filenames moved. They go to `vault/archive/`, which `list` does
    /// not scan — one rename, and recall, the index and the UI are all correct with
    /// no other change. Nothing is destroyed.
    ///
    /// Three conditions, and all three have to hold:
    ///
    /// * **External origin only.** The user's own words are never archived, whatever
    ///   their age. A note you wrote in 2019 is not clutter, it is the thing this
    ///   app exists to keep.
    /// * **Never reinforced.** A reinforce count above zero is a record that the
    ///   user reached for this memory, which is the strongest signal in the vault
    ///   that it should stay.
    /// * **Older than `older_than_days`**, by its own `date`.
    ///
    /// Deliberately *not* keyed on `weight`, which reads like the natural criterion
    /// and is not one: at `DECAY_RATE` 0.015/hour the curve halves in under two days
    /// and hits its 0.12 clamp in about six, so `weight` is very nearly a boolean
    /// for "touched this week" and archiving on it would archive the whole vault.
    pub fn archive_stale(&self, older_than_days: i64) -> Vec<String> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days.max(1));
        let dir = self.vault_path.join("archive");
        let mut moved = vec![];
        for m in self.list() {
            if m.origin != Origin::External || m.reinforce_count > 0 {
                continue;
            }
            // A date we cannot parse is treated as current. Archiving on a parse
            // failure would quietly retire memories for having odd frontmatter.
            let Ok(when) = DateTime::parse_from_rfc3339(&m.date) else { continue };
            if when.with_timezone(&Utc) >= cutoff {
                continue;
            }
            if fs::create_dir_all(&dir).is_ok()
                && fs::rename(self.vault_path.join(&m.filename), dir.join(&m.filename)).is_ok()
            {
                moved.push(m.filename);
            }
        }
        moved.sort();
        moved
    }
}
