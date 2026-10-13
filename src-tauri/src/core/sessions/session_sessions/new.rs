use super::*;

impl Session {
    pub fn new(agent: &str, workspace: &str, model: &str) -> Self {
        let n: u64 = rand::random();
        let t = now();
        Session {
            session_id: format!("s-{:012x}", n & 0xff_ffff_ffff_ff),
            title: String::new(),
            workspace: workspace.to_string(),
            agent: agent.to_string(),
            model: model.to_string(),
            mode: default_mode(),
            created_at: t,
            updated_at: t,
            pinned: false,
            archived: false,
            messages: Vec::new(),
            origin: None,
            parent_id: None,
            compact_from: 0,
            compact_summary: String::new(),
            evolved_at: 0.0,
            dir: String::new(),
            profiled_at: 0.0,
            connections: BTreeMap::new(),
        }
    }

    /// User turns since the last evolution pass — the "is there enough to review"
    /// signal. Counted from `evolved_at` so a second pass never re-reads the first
    /// pass's material.
    pub fn turns_since_evolution(&self) -> usize {
        self.messages.iter().filter(|m| m.role == "user" && m.at > self.evolved_at).count()
    }

    /// What the user themselves has said since the profile learner last looked.
    ///
    /// Only their turns, and deliberately so: this is what the profile is allowed
    /// to be learned from. An assistant turn is our own words coming back, and a
    /// tool result is whatever some third party wrote — teaching either of them to
    /// a store that reaches every future system prompt is the injection path Step 1
    /// closed for the run gate.
    pub fn user_turns_since_profile(&self) -> Vec<String> {
        self.messages
            .iter()
            .filter(|m| m.role == "user" && m.at > self.profiled_at && !m.content.trim().is_empty())
            .map(|m| m.content.clone())
            .collect()
    }

    /// The conversation as plain text for a review pass. Only what was said — a
    /// run's tool chain is not part of what the assistant should learn from.
    pub fn transcript(&self) -> String {
        self.messages
            .iter()
            .filter(|m| (m.role == "user" || m.role == "assistant") && !m.content.trim().is_empty())
            .map(|m| format!("{}: {}", m.role, m.content.trim()))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The history a run should be given: everything after the last `/compact`,
    /// with the summary of what came before carried in front of it as context.
    /// Roles are `user`/`assistant` only — a run's own tool chain is never replayed.
    pub fn history(&self) -> Vec<(String, String)> {
        self.history_wired().into_iter().map(|(r, c, _)| (r, c)).collect()
    }

    /// The same history, keeping each turn's native tool traffic.
    ///
    /// HARNESS-PLAN H2: replaying only `(role, content)` means the model cannot see
    /// that it already read the file, ran the command and worked around the error —
    /// so it does all three again. What the next turn can *use* depends on the model
    /// it is about to run on, which is `core::transcript`'s decision, not this one's.
    pub fn history_wired(&self) -> Vec<(String, String, Option<crate::core::transcript::TurnWire>)> {
        let mut out: Vec<(String, String, Option<crate::core::transcript::TurnWire>)> = Vec::new();
        if !self.compact_summary.is_empty() {
            out.push((
                "user".to_string(),
                format!(
                    "[Summary of the earlier part of this conversation, condensed to save context. Treat it as established.]\n\n{}",
                    self.compact_summary
                ),
                None,
            ));
            out.push((
                "assistant".to_string(),
                "Understood — I have that context.".to_string(),
                None,
            ));
        }
        out.extend(
            self.messages
                .iter()
                .skip(self.compact_from.min(self.messages.len()))
                .filter(|m| m.role == "user" || m.role == "assistant")
                .map(|m| match &m.persona {
                    // Another assistant's reply (an `@` mention): its words, marked as
                    // theirs, and not its tool traffic — those were its tools, not the
                    // next run's.
                    Some(p) => (m.role.clone(), format!("[Answered by @{p}.]\n\n{}", m.content), None),
                    None => (m.role.clone(), m.content.clone(), m.wire.clone()),
                }),
        );
        out
    }

    /// A session with a caller-chosen id (automation runs and Slack threads own
    /// their session id so they can be reopened).
    pub fn with_id(id: &str, agent: &str, workspace: &str, model: &str) -> Self {
        let mut s = Session::new(agent, workspace, model);
        s.session_id = id.to_string();
        s
    }

    /// A short label for the sidebar.
    ///
    /// The raw first line is usually a bad title — it starts with filler ("Can you
    /// take care of these for me?"), or names the plumbing ("Call load_skill with
    /// name project-tracker…"), and several requests that open the same way all
    /// render identically. This strips the polite lead-in and keeps the part that
    /// says what the task IS. Deterministic on purpose: a title that costs a model
    /// call would be slow, and one that changes between runs is worse than a plain
    /// one.
    pub fn derived_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.clone();
        }
        let Some(first) = self.messages.iter().find(|m| m.role == "user") else {
            return "New session".into();
        };
        // Prefer the first non-empty line; if it's only a lead-in ("Can you take
        // care of these for me?"), the first list item says far more.
        let mut lines = first.content.lines().map(str::trim).filter(|l| !l.is_empty());
        let head = lines.next().unwrap_or("");
        let mut text = head.to_string();
        if is_lead_in(head) {
            if let Some(next) = lines.next() {
                text = strip_bullet(next).to_string();
            }
        }
        let text = strip_bullet(&text).to_string();
        let text = strip_prefixes(&text);
        let text = strip_filler(&text);
        // Stop at the first sentence: "Summarise the sheet. Use the one from
        // Tuesday." is one chat about one thing, and the second sentence is
        // instructions rather than subject.
        let text = text.split(['.', '?', '!', '\n']).next().unwrap_or(&text).trim().to_string();
        // …and at the first comma, which is usually where a sentence stops being about
        // its subject: "ask_user with three questions, each with three options" is a
        // chat about three questions. Guarded at three words so a leading aside
        // ("Right, do X") doesn't become the whole name — and it is a real trade:
        // "a brief on Q3, Q4 and Q1" keeps only Q3. A name is a handle, not a
        // summary, and the full text is one hover away.
        let text = match text.split_once(',') {
            Some((head, _)) if head.split_whitespace().count() >= 3 => head.trim().to_string(),
            _ => text,
        };
        let text = strip_prefixes(&text);
        let text = text.trim_end_matches([':', '.', ',', ';', ' ']).trim();
        if text.is_empty() {
            return "New session".into();
        }
        // A PHRASE, not a truncated sentence — see `first_words`.
        //
        // Two caps, and BOTH have to set `cut`. They didn't: the word cap marked its
        // truncation and the character cap did not, so a six-word title one character
        // over the limit lost a word silently — the one case where an ellipsis is the
        // whole point, because it is the only thing saying the name is not the name.
        let (mut out, mut cut) = first_words(text, 6);
        if out.chars().count() > 42 {
            out = out.chars().take(42).collect();
            if let Some(i) = out.rfind(' ') {
                out.truncate(i);
            }
            cut = true;
        }
        let mut out = out.trim_end_matches([':', '.', ',', ';', ' ']).to_string();
        if cut {
            out.push('…');
        }
        let mut c = out.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => "New session".into(),
        }
    }
}
