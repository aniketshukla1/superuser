use super::*;

impl SessionStore {
    pub fn new(data_dir: &Path) -> Self {
        let root = data_dir.join("sessions");
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    /// Where a session lives on disk.
    ///
    /// Session ids come from us or from a connector, so they have to be made
    /// filesystem-safe. Mapping every unsafe character to `_` did that but was
    /// **lossy**: `chat/one` and `chat:one` both became `chat_one`, so the second
    /// conversation opened the first one's transcript. Connector ids are exactly
    /// the ones carrying punctuation, which made this a cross-conversation leak
    /// between, say, two Slack channels rather than a cosmetic clash.
    ///
    /// So the mapping has to be injective. Ids that are already safe are used
    /// as-is — that keeps existing folders readable and the store browsable —
    /// and anything else gets a short hash of the *original* id appended, which
    /// distinguishes ids that would otherwise collapse together.
    /// This conversation's working folder, pinned on first ask.
    ///
    /// The rule, and the reason the field exists: a conversation's folder is
    /// decided once and then never reconsidered. `scratch_base` is where the NEXT
    /// conversation will go, not a statement about every past one — changing it
    /// must not move work that already happened.
    ///
    /// A session that predates the field pins whatever it was already using, so
    /// the upgrade moves nothing. `legacy` is that path, passed in rather than
    /// computed here because `agent_exec` owns the naming.
    pub fn working_dir(&self, id: &str, scratch_base: &str, legacy: PathBuf) -> PathBuf {
        let Some(mut s) = self.get(id) else {
            // No record: a subagent's derived id, or a run that has not saved yet.
            // Nothing to pin to, so it uses the computed path exactly as before.
            return legacy;
        };
        if !s.dir.trim().is_empty() {
            return PathBuf::from(s.dir);
        }
        // Pin it. A folder that already holds WORK wins over the setting — that is
        // what makes "existing conversations keep their current folder" true rather
        // than aspirational.
        //
        // "Holds work" cannot be `legacy.exists()`: the session's own record lives
        // in that same folder, so it exists for every saved session and the setting
        // would never apply to anything. What counts is a file that is not
        // `session.json` — a deliverable someone might have open, or a path already
        // quoted in the transcript.
        let base = scratch_base.trim();
        let chosen = if base.is_empty() || has_deliverables(&legacy) {
            legacy
        } else {
            PathBuf::from(shellexpand_home(base)).join(Self::folder_for(id))
        };
        s.dir = chosen.to_string_lossy().to_string();
        self.save(&s);
        chosen
    }

    pub(in crate::core::sessions) fn dir(&self, id: &str) -> PathBuf {
        let current = self.root.join(Self::folder_for(id));
        if current.exists() {
            return current;
        }
        // Installs that predate the injective naming have their sessions under
        // the old lossy folder name. Adopt one only when the session inside
        // actually claims this id — that keeps existing conversations reachable
        // without re-opening the collision for the ids that used to share a
        // folder, since at most one of them can match.
        if let Some(legacy) = self.legacy_dir(id) {
            return legacy;
        }
        current
    }

    pub(in crate::core::sessions) fn legacy_dir(&self, id: &str) -> Option<PathBuf> {
        let lossy: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        if lossy.is_empty() || lossy == Self::folder_for(id) {
            return None;
        }
        let d = self.root.join(lossy);
        let owner = fs::read_to_string(d.join("session.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Session>(&s).ok())
            .map(|s| s.session_id)?;
        (owner == id).then_some(d)
    }

    pub(crate) fn folder_for(id: &str) -> String {
        let safe = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if !id.is_empty() && id.chars().all(safe) {
            return id.to_string();
        }
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(id.as_bytes());
        let tag: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
        let stem: String = id.chars().map(|c| if safe(c) { c } else { '_' }).take(48).collect();
        // An empty id would otherwise resolve to the store root, and `delete`
        // would then remove every session on the machine.
        if stem.is_empty() {
            format!("s-{tag}")
        } else {
            format!("{stem}-{tag}")
        }
    }

    pub(in crate::core::sessions) fn file(&self, id: &str) -> PathBuf {
        self.dir(id).join("session.json")
    }

    pub fn save(&self, s: &Session) {
        let dir = self.dir(&s.session_id);
        let _ = fs::create_dir_all(&dir);
        // Atomic: a session is written on every message while the sidebar, the
        // evolution scan and the profile learner all read it. This one holds the
        // user's actual conversation, so a truncated write is lost words.
        let _ = crate::core::store::write_json_atomic(&dir.join("session.json"), s);
    }

    pub fn get(&self, id: &str) -> Option<Session> {
        fs::read_to_string(self.file(id)).ok().and_then(|s| serde_json::from_str(&s).ok())
    }

    /// Load-or-create, so a socket can open a session id that doesn't exist yet
    /// (the GUI generates an id, then connects).
    pub fn get_or_create(&self, id: &str, agent: &str, workspace: &str, model: &str) -> Session {
        self.get(id).unwrap_or_else(|| {
            let s = Session::with_id(id, agent, workspace, model);
            self.save(&s);
            s
        })
    }

    /// Every session, newest first.
    pub fn list(&self) -> Vec<Session> {
        let mut out: Vec<Session> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let f = e.path().join("session.json");
                fs::read_to_string(f).ok().and_then(|s| serde_json::from_str::<Session>(&s).ok())
            })
            .collect();
        out.sort_by(|a, b| b.updated_at.partial_cmp(&a.updated_at).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    pub fn append(&self, id: &str, msg: Message) -> Option<Session> {
        let mut s = self.get(id)?;
        s.messages.push(msg);
        s.updated_at = now();
        self.save(&s);
        Some(s)
    }

    /// Delete the session and its working folder (its artifacts live there).
    pub fn delete(&self, id: &str) -> bool {
        // Observations are per-session by construction and must not outlive the
        // session — they are whole command outputs, which is the most sensitive
        // material this app keeps and the least expected to survive a delete.
        // `root` is `<data_dir>/sessions`, so its parent is where the store lives.
        if let Some(data_dir) = self.root.parent() {
            crate::core::observations::ObservationStore::for_session(data_dir, id).clear();
        }
        fs::remove_dir_all(self.dir(id)).is_ok()
    }

    pub fn rename(&self, id: &str, title: &str) -> bool {
        match self.get(id) {
            Some(mut s) => {
                s.title = title.to_string();
                s.updated_at = now();
                self.save(&s);
                true
            }
            None => false,
        }
    }
}
