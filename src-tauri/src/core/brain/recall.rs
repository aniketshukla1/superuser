use super::*;

impl Recall {
    /// True when no store returned anything. The prompt says so plainly rather than
    /// leaving the model to infer emptiness from three missing headings — a model
    /// handed silence tends to fill it.
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
            && self.journal.is_empty()
            && self.pages.is_empty()
            && self.inventories.is_empty()
    }

    /// The category listings, as the model sees them.
    ///
    /// Each one states outright whether it is complete, and a complete one says that
    /// absence from it is evidence — otherwise a model looking at 117 GitHub titles
    /// and asked about a repo not among them will hedge rather than answer, which
    /// wastes the one block in the prompt that can be trusted absolutely.
    pub fn inventory_block(&self) -> String {
        let mut s = String::new();
        for inv in &self.inventories {
            if inv.complete() {
                s.push_str(&format!(
                    "\n[INVENTORY — all {} memories filed under \"{}\", by title. This list is COMPLETE: \
                     use it to answer \"which\", \"list\", \"name all\" and \"do I have\" questions about \
                     this category exactly, and treat absence from it as proof there is no such memory.]\n",
                    inv.total, inv.category
                ));
            } else {
                s.push_str(&format!(
                    "\n[INVENTORY — the {} most recent of {} memories filed under \"{}\", by title. This \
                     list is INCOMPLETE: enumerate from it only while saying it is a partial list, and \
                     never treat absence from it as proof of anything.]\n",
                    inv.titles.len(),
                    inv.total,
                    inv.category
                ));
            }
            for t in &inv.titles {
                s.push_str(&format!("- {t}\n"));
            }
        }
        s
    }

    /// The retrieved material, as the model sees it.
    pub fn block(&self) -> String {
        let budget = self.budget;
        let mut s = String::new();

        if !self.memories.is_empty() {
            s.push_str("\n[MEMORIES — from the user's own connected sources and saved notes]\n");
            for m in &self.memories {
                s.push_str(&format!("\n### {} [{}]", m.title, m.category));
                if !m.date.is_empty() {
                    s.push_str(&format!(" ({})", m.date));
                }
                s.push('\n');
                if !m.summary.is_empty() {
                    s.push_str(&format!("{}\n", m.summary.trim()));
                }
                s.push_str(&format!("{}\n", clip(&m.body, budget.memory_chars)));
            }
        }

        if !self.journal.is_empty() {
            s.push_str("\n[TIMELINE — what was established in past conversations, and when]\n");
            for h in &self.journal {
                match h.tier {
                    Tier::Core => s.push_str(&format!("- (core memory) {}\n", h.text.trim())),
                    Tier::Daily => s.push_str(&format!("- ({}) {}\n", h.date, h.text.trim())),
                }
            }
        }

        if !self.pages.is_empty() {
            s.push_str("\n[KNOWLEDGE — pages written deliberately about a topic]\n");
            for p in &self.pages {
                s.push_str(&format!(
                    "\n### {} [{}] (updated {})\n{}\n",
                    p.title,
                    p.category,
                    p.updated_at,
                    clip(&p.body, budget.page_chars)
                ));
            }
        }

        if s.is_empty() {
            s.push_str("\n(Nothing in the vault matched this question.)\n");
        }
        s
    }

    /// One line naming what came back, for a progress row.
    ///
    /// Counts rather than titles: this is read while waiting, at a glance, and the
    /// titles are already on their way in the answer's own footer.
    pub fn found(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.memories.is_empty() {
            parts.push(format!("{} of {} memories", self.memories.len(), self.census.memories));
        }
        if !self.journal.is_empty() {
            parts.push(format!("{} timeline entries", self.journal.len()));
        }
        if !self.pages.is_empty() {
            parts.push(format!("{} knowledge pages", self.pages.len()));
        }
        for inv in &self.inventories {
            parts.push(if inv.complete() {
                format!("all {} filed under {}", inv.total, inv.category)
            } else {
                format!("{} of {} filed under {}", inv.titles.len(), inv.total, inv.category)
            });
        }
        let mut line = if parts.is_empty() { "nothing matched".to_string() } else { parts.join(", ") };
        if let Some(why) = &self.degraded {
            line.push_str(&format!(" — keyword matching only: {why}"));
        }
        line
    }

    /// The citation footer, built from what was actually retrieved.
    ///
    /// Deterministic on purpose. Asking the model to cite its sources produces
    /// plausible titles for pages that do not exist, and a made-up citation is worse
    /// than none: it is the part of the answer the user would have used to check the
    /// rest. Empty when nothing was retrieved, so an "I don't have that" answer is
    /// not followed by a sources heading over an empty list.
    pub fn sources_block(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut s = String::from("\n\n---\n**Where this came from**\n");
        // First, because it is the strongest claim on the page: a complete listing
        // means the answer enumerated rather than sampled, and the reader should be
        // told which of those they are reading.
        for inv in &self.inventories {
            if inv.complete() {
                s.push_str(&format!(
                    "- **Inventory** — all {} memories filed under {}\n",
                    inv.total, inv.category
                ));
            } else {
                s.push_str(&format!(
                    "- **Inventory** — {} of {} memories filed under {} (partial list)\n",
                    inv.titles.len(),
                    inv.total,
                    inv.category
                ));
            }
        }
        if !self.memories.is_empty() {
            let titles: Vec<String> =
                self.memories.iter().map(|m| format!("{} ({})", m.title, m.category)).collect();
            s.push_str(&format!("- **Memories** — {}\n", titles.join("; ")));
        }
        if !self.pages.is_empty() {
            let titles: Vec<&str> = self.pages.iter().map(|p| p.title.as_str()).collect();
            s.push_str(&format!("- **Knowledge** — {}\n", titles.join("; ")));
        }
        if !self.journal.is_empty() {
            // Dates, deduplicated and kept in the order recall ranked them: a list
            // that repeats "2026-08-12" four times says the journal was searched
            // badly, not that four things happened that day.
            let mut dates: Vec<&str> = Vec::new();
            let mut core = false;
            for h in &self.journal {
                match h.tier {
                    Tier::Core => core = true,
                    Tier::Daily => {
                        if !dates.contains(&h.date.as_str()) {
                            dates.push(&h.date);
                        }
                    }
                }
            }
            let mut parts: Vec<String> = Vec::new();
            if core {
                parts.push("core memory".into());
            }
            if !dates.is_empty() {
                parts.push(dates.join(", "));
            }
            s.push_str(&format!("- **Timeline** — {}\n", parts.join("; ")));
        }
        s
    }
}
