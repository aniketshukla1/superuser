use super::*;

impl Session {
    /// The summary shape the GUI's sidebar reads.
    pub fn info(&self) -> Value {
        serde_json::json!({
            "session_id": self.session_id,
            "title": self.derived_title(),
            // Whether the title was CHOSEN or guessed from the first thing said.
            //
            // `derived_title` is a good guess and the sidebar wants it — a list of
            // "New session" rows is unscannable. But the chat's own heading is a
            // different job: showing a fragment of your own prompt back to you at
            // heading size tells you nothing you don't know, and a long one turns
            // the top of the page into a wall. The topbar reads this and falls back
            // to the persona's name.
            "named": !self.title.trim().is_empty(),
            // What was actually asked, for the row's tooltip.
            //
            // The name is a handle now — six words, first clause — which is what
            // makes a column of them scannable and also what makes it lossy. The
            // comment justifying that cut says the full text is one hover away, so
            // it had better be.
            "preview": self.messages.iter().find(|m| m.role == "user")
                .map(|m| m.content.chars().take(280).collect::<String>())
                .unwrap_or_default(),
            "workspace": self.workspace,
            "agent": self.agent,
            "model": self.model,
            "mode": self.mode,
            "updated_at": iso(self.updated_at),
            "messages": self.messages.len(),
            "pinned": self.pinned,
            "archived": self.archived,
            "origin": self.origin,
            "parent_id": self.parent_id,
        })
    }
}
