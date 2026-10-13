
/// The mention that routes a message here. Lower-case; matching is case-insensitive.
pub const MENTION: &str = "@brain";

/// What a question is allowed to pull out of each store, and how much of each hit
/// survives into the prompt.
///
/// Two presets rather than one, because the ceiling is not a matter of taste: a
/// 7B model with an 8k window and a frontier model with 200k cannot be handed the
/// same twenty thousand characters of vault. Sending the cloud budget to a local
/// model does not produce a worse answer, it produces a truncated prompt and an
/// answer about whatever happened to survive the cut — which reads exactly like the
/// brain having forgotten something it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub memories: usize,
    pub memory_chars: usize,
    pub journal: usize,
    pub pages: usize,
    pub page_chars: usize,
    /// Ceiling on titles listed across every matched category (see `Inventory`).
    ///
    /// A guardrail, not a budget knob. Titles are short, so the ordinary case fits
    /// whole and the list stays exhaustive — which is the entire point of it. This
    /// exists so a vault with fifty thousand memories in one category cannot blow
    /// the context window, not to routinely trim.
    pub inventory_titles: usize,
    /// Floor for the answer itself. Never lowers a user's own higher `max_tokens`:
    /// a detailed answer is the point, and truncating it mid-list is the one failure
    /// the user cannot work around from their side.
    pub answer_tokens: u32,
}

impl Budget {
    /// A local model: fewer sources, shorter excerpts, still all four stores.
    pub const LOCAL: Budget = Budget {
        memories: 8,
        memory_chars: 600,
        journal: 8,
        pages: 3,
        page_chars: 900,
        inventory_titles: 200,
        answer_tokens: 1600,
    };

    /// A cloud model: the question gets the whole shelf.
    pub const CLOUD: Budget = Budget {
        memories: 16,
        memory_chars: 1200,
        journal: 14,
        pages: 6,
        page_chars: 2400,
        inventory_titles: 1000,
        answer_tokens: 4096,
    };

    pub fn resolve(local: bool) -> Budget {
        if local {
            Budget::LOCAL
        } else {
            Budget::CLOUD
        }
    }

    /// The budget to use once a category is being listed in full.
    ///
    /// An inventory is not extra material, it is *different* material about the same
    /// thing — and it makes the excerpts largely redundant. A question that names a
    /// category is enumerative, so the eight 600-character excerpts of that same
    /// category are paying a second time to say less: the titles already name every
    /// one of them, exhaustively, at a twelfth of the cost.
    ///
    /// So the inventory displaces the excerpts rather than stacking on top of them,
    /// which keeps the prompt roughly the size it was before this existed. That
    /// matters far more than it sounds: measured on a local 12B, adding ~1,100
    /// tokens of titles on top of a full excerpt block took a 2½-minute answer past
    /// ten minutes. A few excerpts stay, because a question can name a category and
    /// still turn on what one particular entry *said*.
    pub(super) fn displaced(self) -> Budget {
        Budget {
            memories: (self.memories / 3).max(2),
            memory_chars: self.memory_chars / 2,
            ..self
        }
    }
}

/// Counts over the *whole* vault, not over what was retrieved.
///
/// This is what makes "how many", "which categories", "how far back does this go"
/// answerable at all. Retrieval returns the twelve most relevant memories whatever
/// the question, so a model asked to count from them will confidently answer twelve.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Census {
    pub memories: usize,
    pub categories: Vec<(String, usize)>,
    /// Earliest and latest memory dates present, `""` when the vault is empty.
    pub oldest: String,
    pub newest: String,
    pub journal_days: usize,
    pub has_core_memory: bool,
    pub knowledge_pages: usize,
    pub profile_facets: usize,
}

impl Census {
    /// The census as the model sees it. Rendered rather than serialized so the
    /// wording carries the guarantee: these are exact, the excerpts below are not
    /// exhaustive.
    pub fn block(&self) -> String {
        if self.memories == 0
            && self.journal_days == 0
            && self.knowledge_pages == 0
            && self.profile_facets == 0
        {
            return "The brain is empty — nothing has been saved to it yet.".into();
        }
        let mut s = String::from("[VAULT CENSUS — exact counts over everything, use these for any counting or \"how many\" question]\n");
        s.push_str(&format!("- Memories: {}\n", self.memories));
        if !self.categories.is_empty() {
            s.push_str("- By category:\n");
            for (cat, n) in &self.categories {
                s.push_str(&format!("  - {cat}: {n}\n"));
            }
        }
        if !self.oldest.is_empty() {
            s.push_str(&format!("- Memory date range: {} to {}\n", self.oldest, self.newest));
        }
        s.push_str(&format!(
            "- Journal: {} daily entries{}\n",
            self.journal_days,
            if self.has_core_memory { ", plus distilled core memory" } else { "" }
        ));
        s.push_str(&format!("- Knowledge pages: {}\n", self.knowledge_pages));
        s.push_str(&format!("- Profile facets held about the user: {}\n", self.profile_facets));
        s
    }
}

/// Every memory in one category, by title.
///
/// The census answers "how many"; this answers "which ones". They are the same
/// problem — a question about the *set* rather than about its most relevant few —
/// and top-k retrieval cannot answer either. Asked which repositories they had
/// starred, a vault holding twenty such memories returned none of them, because the
/// eight retrieval slots went to entries that scored higher on the sentence. The
/// model then said, correctly and uselessly, that it had no record of any.
///
/// No excerpt, no body, no score: just the titles, which for the memories ingestion
/// writes ("Starred owner/repo", "Pushed 3 commit(s) to owner/repo") *are* the
/// content. Twenty times a title is a list; twenty times a memory is the context
/// window.
#[derive(Debug, Clone, PartialEq)]
pub struct Inventory {
    pub category: String,
    /// Newest first, capped by `Budget::inventory_titles`.
    pub titles: Vec<String>,
    /// How many memories the category actually holds.
    pub total: usize,
}

impl Inventory {
    /// Whether every title is present. The distinction is the whole value of this:
    /// an exhaustive list can be enumerated from, and a trimmed one presented as
    /// exhaustive is a confident wrong answer — the exact failure the rest of this
    /// module is built to avoid. So the prompt is told which it is holding.
    pub fn complete(&self) -> bool {
        self.titles.len() == self.total
    }
}
