use super::*;

impl Ledger {
    pub fn new(vault: &Path) -> Self {
        Self { root: vault.to_path_buf() }
    }

    /// Run git in the vault. `-c` flags keep our commits independent of the user's
    /// global setup: no signing prompt on a machine with `commit.gpgsign`, and no
    /// identity error on one with no `user.email` configured.
    pub(super) fn git(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["-c", "user.name=Superuser", "-c", "user.email=superuser@localhost", "-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .map_err(|e| format!("git unavailable: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// Is there a repository here yet?
    pub fn initialized(&self) -> bool {
        self.root.join(".git").exists()
    }

    /// Create the repository if it isn't one already. A vault the user has already
    /// put under git (an Obsidian vault often is) is adopted as-is — this never
    /// re-initializes, and never touches an existing `.gitignore`.
    pub(super) fn ensure_repo(&self) -> Result<(), String> {
        if self.initialized() {
            return Ok(());
        }
        if !self.root.exists() {
            return Err("the vault doesn't exist yet".into());
        }
        self.git(&["init", "--quiet"])?;
        let ignore = self.root.join(".gitignore");
        if !ignore.exists() {
            // Machine noise and the consolidation's own bookkeeping. Everything
            // else in the vault is content and belongs in the history.
            let _ = std::fs::write(&ignore, ".DS_Store\njournal/.dream-state\n");
        }
        Ok(())
    }

    /// Files matching one source, as a git pathspec. Vault filenames are
    /// `<connector>-<id>.md` by convention across every integration, so the prefix
    /// *is* the attribution. `*` means the whole vault.
    pub(super) fn pathspec(source: &str) -> String {
        if source == "*" || source.trim().is_empty() {
            ".".to_string()
        } else {
            format!("{source}-*.md")
        }
    }

    pub(super) fn marker_ref(source: &str) -> String {
        format!("refs/superuser/read/{}", source.trim())
    }

    pub(super) fn head(&self) -> Option<String> {
        self.git(&["rev-parse", "HEAD"]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    /// Commit whatever changed, attributing it to `source`. Returns the new commit
    /// id, or `None` when the working tree was already clean — a sync that fetched
    /// nothing new leaves no empty commit behind.
    pub fn snapshot(&self, source: &str, trigger: &str) -> Result<Option<String>, String> {
        self.ensure_repo()?;
        self.git(&["add", "-A"])?;
        // Nothing staged → nothing happened. `diff --cached --quiet` exits non-zero
        // when there *are* staged changes, so an Err here is the interesting case.
        if self.git(&["diff", "--cached", "--quiet"]).is_ok() {
            return Ok(None);
        }
        // Trailers, so a later reader can tell an automatic sync from a hand edit
        // without parsing prose.
        let msg = format!("sync: {source}\n\nSource-Id: {source}\nTrigger: {trigger}\n");
        // --no-verify: a vault that is also the user's own git repo may carry hooks,
        // and a background sync must not be able to run them.
        self.git(&["commit", "--no-verify", "--quiet", "-m", &msg])?;
        Ok(self.head())
    }

    /// Everything that changed for `source` since the last read, moving the marker
    /// to HEAD unless `commit` is false.
    ///
    /// With no marker yet, the baseline is the empty tree: the first call reports
    /// the source's whole contents, which is the honest answer to "what's new to
    /// me" when the answer is "all of it".
    pub fn changes_since_read(&self, source: &str, commit: bool) -> Result<Changes, String> {
        if !self.initialized() {
            return Err("no history yet — nothing has been synced since the ledger was added".into());
        }
        let source = source.trim().to_string();
        let Some(head) = self.head() else {
            return Err("the vault has no commits yet".into());
        };
        let marker = self
            .git(&["rev-parse", "--verify", "--quiet", &Self::marker_ref(&source)])
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| EMPTY_TREE.to_string());

        let spec = Self::pathspec(&source);
        let raw = self.git(&["diff", "--name-status", &format!("{marker}..{head}"), "--", &spec])?;

        let mut out = Changes { source: source.clone(), ..Default::default() };
        for line in raw.lines() {
            let mut cols = line.split('\t');
            let (Some(status), Some(path)) = (cols.next(), cols.next_back()) else { continue };
            let name = path.trim().to_string();
            match status.chars().next() {
                Some('A') => out.added.push(name),
                Some('D') => out.removed.push(name),
                // R (renamed) and C (copied) land here with their destination path,
                // which is the file that exists now — the useful one to report.
                Some(_) => out.modified.push(name),
                None => {}
            }
        }
        out.total = self
            .git(&["ls-files", "--", &spec])
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0);

        if commit && !out.is_empty() {
            // Only acknowledge when there was something to acknowledge, so a preview
            // that happens to be empty can't quietly consume a later delta.
            self.git(&["update-ref", &Self::marker_ref(&source), &head])?;
            out.acknowledged = true;
        }
        Ok(out)
    }

    /// Mark everything up to HEAD as seen for `source`, without reading a diff.
    ///
    /// The two-step counterpart to a `peek`: a caller that has to *do* something
    /// with a delta before it can honestly claim to have seen it — evaluate it with
    /// a model, say — peeks first and acknowledges only once that succeeded. A tick
    /// that failed must leave the marker where it was, or the items it never managed
    /// to look at are silently skipped forever.
    pub fn acknowledge(&self, source: &str) -> Result<(), String> {
        let Some(head) = self.head() else {
            return Err("the vault has no commits yet".into());
        };
        self.git(&["update-ref", &Self::marker_ref(source.trim()), &head]).map(|_| ())
    }

    /// Sources that have ever been read, with the commit each was last read at.
    /// Useful for showing the ledger's state; not needed to use it.
    pub fn read_markers(&self) -> Vec<(String, String)> {
        let Ok(raw) = self.git(&["for-each-ref", "--format=%(refname:lstrip=3) %(objectname)", "refs/superuser/read"]) else {
            return vec![];
        };
        raw.lines()
            .filter_map(|l| l.split_once(' '))
            .map(|(name, sha)| (name.to_string(), sha.trim().to_string()))
            .collect()
    }

    /// How many snapshots the vault has. Zero means the ledger exists but nothing
    /// has been recorded through it yet.
    pub fn snapshot_count(&self) -> usize {
        self.git(&["rev-list", "--count", "HEAD"]).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
    }
}
