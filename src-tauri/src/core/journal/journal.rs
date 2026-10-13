use super::*;

impl Journal {
    /// `vault/journal`. Nothing is created until something is written.
    pub fn new(vault: &Path) -> Self {
        Self { root: vault.join("journal") }
    }

    pub fn core_path(&self) -> PathBuf {
        self.root.join("MEMORY.md")
    }
    pub fn daily_path(&self, date: &str) -> PathBuf {
        self.root.join("daily").join(format!("{date}.md"))
    }
    pub fn recap_path(&self, date: &str) -> PathBuf {
        self.root.join("recaps").join(format!("{date}.md"))
    }
    pub fn dream_path(&self, date: &str) -> PathBuf {
        self.root.join("dreams").join(format!("{date}.md"))
    }

    pub fn read_core(&self) -> String {
        fs::read_to_string(self.core_path()).unwrap_or_default()
    }

    pub fn write_core(&self, body: &str) -> std::io::Result<()> {
        let _ = fs::create_dir_all(&self.root);
        crate::core::store::write_atomic(&self.core_path(), &(body.trim_end().to_string() + "\n"))
    }

    /// Append one timestamped block to a dated file, creating it only now. Shared by
    /// both tiers so evidence and interpretation stay in the same shape and can be
    /// read back by the same splitter.
    pub(super) fn append_dated(&self, path: PathBuf, date: &str, title: &str, text: &str, source: &str) -> std::io::Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let stamp = chrono::Local::now().format("%H:%M").to_string();
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let mut out = if existing.is_empty() { format!("# {title}\n") } else { existing };
        if !out.ends_with('\n') {
            out.push('\n');
        }
        // The session id rides in the heading rather than the body: it is provenance,
        // and it keeps the block itself pure evidence. Without it a daily entry is a
        // quotation with no way back to the full record.
        let head = if source.trim().is_empty() { stamp } else { format!("{stamp} · {}", source.trim()) };
        out.push_str(&format!("\n## {head}\n\n{text}\n"));
        let _ = date; // the file is already keyed by date; kept for call-site clarity
        fs::write(&path, out)
    }

    /// Record what was actually said, as it falls out of the context window.
    ///
    /// `text` is a transcript excerpt, not a summary — see the module docs. `source`
    /// is the session it came from, so any line here can be traced back to the full
    /// conversation on disk.
    pub fn append_daily(&self, date: &str, text: &str, source: &str) -> std::io::Result<()> {
        self.append_dated(self.daily_path(date), date, date, text, source)
    }

    /// Record the model's reading of those same turns, beside the evidence.
    pub fn append_recap(&self, date: &str, text: &str, source: &str) -> std::io::Result<()> {
        self.append_dated(self.recap_path(date), date, &format!("Recaps — {date}"), text, source)
    }

    /// Write both tiers for one context-trim event — the only moment either is
    /// written, and the reason they cannot drift apart.
    ///
    /// `dropped` is the messages that fell out of the window; their prose becomes
    /// the daily entry, bounded head-and-tail so a day of long turns doesn't turn
    /// the journal into a second copy of every session (the session file on disk is
    /// the complete record, and the heading names it). `summary` is the model's
    /// recap of the same messages and goes to the other file. A summary with no
    /// evidence behind it is not written at all.
    pub fn record_trim(&self, date: &str, dropped: &[serde_json::Value], summary: &str, session: &str) {
        let evidence = crate::core::context::middle_out(&crate::core::context::transcript(dropped), EVIDENCE_CHAR_CAP);
        if evidence.trim().is_empty() {
            return;
        }
        let _ = self.append_daily(date, &evidence, session);
        let _ = self.append_recap(date, summary, session);
    }

    pub fn read_daily(&self, date: &str) -> Option<String> {
        fs::read_to_string(self.daily_path(date)).ok()
    }

    pub(super) fn list_dir(&self, dir: &Path) -> Vec<Entry> {
        let mut out: Vec<Entry> = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "md").unwrap_or(false) {
                    let date = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if let Ok(body) = fs::read_to_string(&p) {
                        out.push(Entry { date, body });
                    }
                }
            }
        }
        // Filenames are ISO dates, so lexicographic order is chronological.
        out.sort_by(|a, b| b.date.cmp(&a.date));
        out
    }

    /// Daily files, newest first. The evidence tier.
    pub fn dailies(&self) -> Vec<Entry> {
        self.list_dir(&self.root.join("daily"))
    }

    /// Recap files, newest first. The interpretation tier — readable, and
    /// deliberately not what consolidation or recall is built on.
    pub fn recaps(&self) -> Vec<Entry> {
        self.list_dir(&self.root.join("recaps"))
    }

    /// Dream diaries, newest first.
    pub fn dreams(&self) -> Vec<Entry> {
        self.list_dir(&self.root.join("dreams"))
    }

    /// Self-evolution records, newest first — what each review pass changed.
    pub fn evolutions(&self) -> Vec<Entry> {
        self.list_dir(&self.root.join("evolution"))
    }

    /// The last `days` daily files that actually exist, newest first. Quiet days
    /// simply aren't there — this never fabricates empty ones.
    pub fn recent_dailies(&self, days: usize) -> Vec<Entry> {
        self.dailies().into_iter().take(days).collect()
    }

    pub fn write_dream(&self, date: &str, body: &str) -> std::io::Result<()> {
        let path = self.dream_path(date);
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        // Several dreams in one day append rather than overwrite: a manual run must
        // not erase the record of the nightly one.
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let stamp = chrono::Local::now().format("%H:%M").to_string();
        let mut out = if existing.is_empty() { format!("# Dreams — {date}\n") } else { existing };
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("\n## {stamp}\n\n{}\n", body.trim()));
        fs::write(&path, out)
    }

    /// The core-memory block for a system prompt. Empty when there's nothing to say,
    /// so a fresh install costs no tokens.
    pub fn system_block(&self) -> String {
        let core = self.read_core();
        let core = core.trim();
        if core.is_empty() {
            return String::new();
        }
        let core: String = if core.chars().count() > CORE_INJECT_CHAR_CAP {
            core.chars().take(CORE_INJECT_CHAR_CAP).collect::<String>() + "\n…"
        } else {
            core.to_string()
        };
        format!(
            "\n\n[LONG-TERM MEMORY — durable facts, preferences and decisions from past conversations. \
             Treat as established and act on it without mentioning this block. If something here is \
             contradicted by what the user says now, the user is right.]\n{core}"
        )
    }

    /// Fingerprint of the material a distillation would read. Stored after a run so
    /// a scheduled dream over unchanged input is skipped instead of paying for an
    /// LLM call to reproduce the same file.
    pub fn input_fingerprint(&self, lookback_days: usize) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.read_core().hash(&mut h);
        for e in self.recent_dailies(lookback_days) {
            e.date.hash(&mut h);
            e.body.hash(&mut h);
        }
        h.finish()
    }

    pub(super) fn state_path(&self) -> PathBuf {
        self.root.join(".dream-state")
    }

    pub fn last_fingerprint(&self) -> Option<u64> {
        fs::read_to_string(self.state_path()).ok()?.trim().parse().ok()
    }

    pub fn record_fingerprint(&self, fp: u64) {
        let _ = fs::create_dir_all(&self.root);
        let _ = crate::core::store::write_atomic(&self.state_path(), &fp.to_string());
    }
}
