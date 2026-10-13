use super::*;

impl SessionStore {
    pub fn set_flags(&self, id: &str, pinned: Option<bool>, archived: Option<bool>) -> bool {
        match self.get(id) {
            Some(mut s) => {
                if let Some(p) = pinned {
                    s.pinned = p;
                }
                if let Some(a) = archived {
                    s.archived = a;
                }
                self.save(&s);
                true
            }
            None => false,
        }
    }

    /// Write a completed scheduled run into the conversation it was set up from,
    /// as a message pair the user can reply to ("expand on the second point").
    ///
    /// Only the *answer* comes back — the run's own planning and tool calls stay in
    /// its isolated session, or a five-minute monitor would bury the conversation.
    /// Older injected pairs are pruned so a frequent task can't crowd out the
    /// user's own messages; nothing the user wrote is ever pruned.
    pub fn inject_scheduled(&self, id: &str, task_id: &str, title: &str, answer: &str, keep_pairs: usize) -> bool {
        let Some(mut s) = self.get(id) else { return false };
        let tag = |m: &Message| m.items.iter().any(|i| i["kind"] == "scheduled");

        let mut ask = Message::new("user", format!("[scheduled] {title}"));
        ask.items = vec![serde_json::json!({ "kind": "scheduled", "task": task_id })];
        let mut reply = Message::new("assistant", answer.to_string());
        reply.items = ask.items.clone();
        s.messages.push(ask);
        s.messages.push(reply);

        // Keep the most recent `keep_pairs` injected pairs, oldest-first removal.
        let injected: Vec<usize> = s.messages.iter().enumerate().filter(|(_, m)| tag(m)).map(|(i, _)| i).collect();
        let excess = injected.len().saturating_sub(keep_pairs * 2);
        if excess > 0 {
            let drop: std::collections::HashSet<usize> = injected.into_iter().take(excess).collect();
            let mut i = 0;
            s.messages.retain(|_| {
                let keep = !drop.contains(&i);
                i += 1;
                keep
            });
        }
        s.updated_at = now();
        self.save(&s);
        true
    }

    /// Stamp a session as reviewed, so the next pass only counts turns after it.
    /// Recorded whatever the pass concluded — a review that found nothing still
    /// looked, and re-reading the same turns every minute would be waste.
    pub fn mark_evolved(&self, id: &str) {
        if let Some(mut s) = self.get(id) {
            s.evolved_at = now();
            self.save(&s);
        }
    }

    /// Stamped *before* a profile pass, like `mark_evolved`: a pass that fails or
    /// finds nothing must not re-read the same turns every minute forever.
    pub fn mark_profiled(&self, id: &str) {
        if let Some(mut s) = self.get(id) {
            s.profiled_at = now();
            self.save(&s);
        }
    }

    /// Record a compaction point: from now on, runs are told `summary` in place of
    /// the first `from` messages. The transcript is untouched, so this is purely
    /// additive and can be undone by compacting back to 0 with an empty summary.
    /// Record a per-session grant for one connector. `clear` drops the override so
    /// the session inherits the persona default again. `enabled`/`scope` are each
    /// optional so the two controls can move independently — flipping a source off
    /// must not silently reset the scope the user chose for it.
    pub fn set_connection(
        &self,
        id: &str,
        connector: &str,
        enabled: Option<bool>,
        scope: Option<&str>,
        clear: bool,
    ) -> bool {
        let Some(mut s) = self.get(id) else { return false };
        if clear {
            s.connections.remove(connector);
        } else {
            let g = s.connections.entry(connector.to_string()).or_default();
            if let Some(v) = enabled {
                g.enabled = v;
            }
            if let Some(v) = scope {
                g.scope = normalize_scope(v);
            }
        }
        s.updated_at = now();
        self.save(&s);
        true
    }

    /// Whether this session may WRITE through `connector`.
    ///
    /// `None` means the session has settled nothing for it, and the caller keeps
    /// whatever it did before scopes existed. Only a grant the user actually made
    /// can narrow a send — an absent one never does.
    pub fn may_write(&self, id: &str, connector: &str) -> Option<bool> {
        self.get(id)?.connections.get(connector).map(|g| g.writable())
    }

    pub fn set_compaction(&self, id: &str, from: usize, summary: &str) -> bool {
        match self.get(id) {
            Some(mut s) => {
                s.compact_from = from.min(s.messages.len());
                s.compact_summary = summary.to_string();
                s.updated_at = now();
                self.save(&s);
                true
            }
            None => false,
        }
    }

    /// True when a session has nothing in it — no turns and no files. Opening the
    /// app connects a socket per session id, so these would otherwise pile up as
    /// blank rows in the sidebar.
    pub fn is_empty(&self, s: &Session) -> bool {
        if !s.messages.is_empty() || !s.title.trim().is_empty() || s.pinned {
            return false;
        }
        // Any artifact means real work happened, even with no recorded turn.
        let dir = self.dir(&s.session_id);
        fs::read_dir(&dir)
            .map(|rd| {
                !rd.flatten().any(|e| {
                    let p = e.path();
                    p.file_name().and_then(|n| n.to_str()) != Some("session.json")
                        && (p.is_file() || p.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false))
                })
            })
            .unwrap_or(true)
    }

    /// Drop empty sessions. Called at startup, when nothing is in flight — a live
    /// session would be mid-turn and is never pruned from under the user.
    pub fn prune_empty(&self) -> usize {
        let mut n = self.list().iter().filter(|s| self.is_empty(s)).filter(|s| self.delete(&s.session_id)).count();
        // Folders with no session.json at all — left by earlier builds that created
        // a working dir just to read it.
        if let Ok(rd) = fs::read_dir(&self.root) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && !p.join("session.json").exists() && p.read_dir().map(|mut d| d.next().is_none()).unwrap_or(false) && fs::remove_dir(&p).is_ok() {
                    n += 1;
                }
            }
        }
        n
    }

    /// The session's working folder — where its deliverables land.
    pub fn workdir(&self, id: &str) -> PathBuf {
        let d = self.dir(id).join("work");
        let _ = fs::create_dir_all(&d);
        d
    }
}
