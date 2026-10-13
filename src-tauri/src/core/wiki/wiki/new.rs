use super::*;

impl Wiki {
    /// Under the journal rather than beside it: this is a record of the assistant's
    /// own history, and it must sit deeper than the vault root that
    /// `MemoryEngine::list` scans, or every pattern would also appear as a memory.
    pub fn new(vault: &Path) -> Self {
        Self { root: Journal::new(vault).root.join("wiki") }
    }

    pub fn patterns_dir(&self) -> PathBuf {
        self.root.join("patterns")
    }
    pub fn index_path(&self) -> PathBuf {
        self.root.join("index.md")
    }
    pub fn logs_path(&self) -> PathBuf {
        self.root.join("logs.md")
    }
    pub fn impact_path(&self) -> PathBuf {
        self.root.join("skill-impact.md")
    }

    pub(in crate::core::wiki) fn pattern_path(&self, id: &str) -> PathBuf {
        self.patterns_dir().join(format!("{id}.md"))
    }

    /// Ids are filenames, and the reviewer chooses them, so they are untrusted.
    /// Slugifying rather than rejecting means a reviewer that writes a title where
    /// an id belongs still gets its pattern stored.
    pub fn safe_id(raw: &str) -> String {
        crate::core::knowledge::slugify(raw)
    }

    pub fn read_pattern(&self, id: &str) -> Option<Pattern> {
        let text = std::fs::read_to_string(self.pattern_path(id)).ok()?;
        Some(parse_pattern(id, &text))
    }

    /// Write a pattern, keeping the bookkeeping of any page already there.
    ///
    /// A second sighting is an *update with a higher `seen`*, never a second page.
    /// Without that, three conversations hitting the same wall leave three
    /// near-identical pages and the reviewer loses the one signal — recurrence —
    /// that tells it which wall is worth building a skill around.
    pub fn upsert_pattern(&self, id: &str, title: &str, kind: Kind, body: &str) -> io::Result<Wrote> {
        let id = Self::safe_id(id);
        let prior = self.read_pattern(&id);
        let (seen, first_seen, wrote) = match &prior {
            Some(p) => (p.seen.saturating_add(1), p.first_seen.clone(), Wrote::Reinforced),
            None => (1, today(), Wrote::Created),
        };
        // Single-line, or `serde_yaml_ng` emits a block scalar and the frontmatter
        // stops being frontmatter.
        let title = if title.trim().is_empty() {
            prior.as_ref().map(|p| p.title.clone()).unwrap_or_else(|| id.replace('-', " "))
        } else {
            title.replace(['\n', '\r'], " ").trim().to_string()
        };
        let text = format!(
            "---\ntitle: {}\nkind: {}\nseen: {}\nfirst_seen: {}\nupdated: {}\n---\n\n{}\n",
            serde_yaml_ng::to_string(&title).map(|s| s.trim().to_string()).unwrap_or_else(|_| title.clone()),
            kind.as_str(),
            seen,
            first_seen,
            today(),
            body.trim(),
        );
        std::fs::create_dir_all(self.patterns_dir())?;
        write_atomic(&self.pattern_path(&id), &text)?;
        Ok(wrote)
    }

    /// Every pattern, most-reinforced first — the order the reviewer should read
    /// them in, since recurrence is what earns attention.
    pub fn patterns(&self) -> Vec<Pattern> {
        let mut out: Vec<Pattern> = std::fs::read_dir(self.patterns_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
            .filter_map(|e| {
                let id = e.path().file_stem()?.to_string_lossy().into_owned();
                let text = std::fs::read_to_string(e.path()).ok()?;
                Some(parse_pattern(&id, &text))
            })
            .collect();
        out.sort_by(|a, b| b.seen.cmp(&a.seen).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// Rebuild `index.md` from the pattern files. Generated rather than edited, so
    /// a page that is deleted or renamed by hand cannot leave a dangling row.
    pub fn rewrite_index(&self) -> io::Result<()> {
        let ps = self.patterns();
        let mut s = String::from(
            "# Wiki index\n\nWhat past reviews have learned about how the work goes. \
             Generated from `patterns/` — edit a pattern, not this file.\n\n",
        );
        if ps.is_empty() {
            s.push_str("_Nothing learned yet._\n");
        } else {
            s.push_str("| pattern | kind | seen | updated |\n|---|---|---|---|\n");
            for p in &ps {
                s.push_str(&format!(
                    "| [{}](patterns/{}.md) — {} | {} | {}× | {} |\n",
                    cell(&p.id),
                    cell(&p.id),
                    cell(&p.title),
                    p.kind.as_str(),
                    p.seen,
                    cell(&p.updated),
                ));
            }
        }
        std::fs::create_dir_all(&self.root)?;
        write_atomic(&self.index_path(), &s)
    }

    /// Append one line to the evolution log. This is the only record that a pass
    /// ran at all when it changed nothing the user can see, which is most passes.
    pub fn log(&self, line: &str) -> io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let existing = std::fs::read_to_string(self.logs_path()).unwrap_or_else(|_| {
            "# Evolution log\n\nOne line per review pass, including the passes that changed nothing.\n".into()
        });
        write_atomic(&self.logs_path(), &format!("{}\n- `{}` — {}\n", existing.trim_end(), stamp(), cell(line)))
    }

    pub(in crate::core::wiki) fn impact_header() -> String {
        "# Skill impact\n\nEvery change a review proposed and what became of it. Rows are kept even when \
         the change was undone — an `undone` row is how the next review knows not to propose it again.\n\n\
         | when | target | patterns | verdict | snapshot | note |\n|---|---|---|---|---|---|\n"
            .into()
    }

    pub fn record_impact(&self, im: &Impact) -> io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let existing = std::fs::read_to_string(self.impact_path()).unwrap_or_else(|_| Self::impact_header());
        let row = format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            cell(&im.at),
            cell(&im.target),
            cell(&im.patterns.join(" ")),
            im.verdict.as_str(),
            cell(&im.snapshot),
            cell(&im.note),
        );
        write_atomic(&self.impact_path(), &format!("{}\n{row}", existing.trim_end_matches('\n')))
    }

    pub fn impacts(&self) -> Vec<Impact> {
        let Ok(text) = std::fs::read_to_string(self.impact_path()) else { return Vec::new() };
        text.lines().filter_map(parse_impact_row).collect()
    }

    /// Flip every row belonging to a snapshot to `undone`, in place.
    ///
    /// Called from the undo path, and the reason the undo is not symmetrical: the
    /// skill goes back to what it was, and the wiki gains the knowledge that this
    /// edit was refused. Returns how many rows changed.
    pub fn mark_undone(&self, snapshot: &str) -> usize {
        let Ok(text) = std::fs::read_to_string(self.impact_path()) else { return 0 };
        let mut n = 0;
        let out: Vec<String> = text
            .lines()
            .map(|line| {
                match parse_impact_row(line) {
                    Some(im) if im.snapshot == snapshot && im.verdict == Verdict::Accepted => {
                        n += 1;
                        // Replace only the verdict cell; rewriting the row from the
                        // parsed struct would silently drop any escaping in it.
                        replace_verdict(line, Verdict::Undone)
                    }
                    _ => line.to_string(),
                }
            })
            .collect();
        if n > 0 {
            let _ = write_atomic(&self.impact_path(), &format!("{}\n", out.join("\n")));
        }
        n
    }
}
