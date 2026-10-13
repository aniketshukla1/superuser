use super::*;

/// Categories named by the question, in census order.
///
/// A plain containment test on the category name, with the match required to stand
/// as its own word — "Chat" must not fire on "chatter" — but tolerant of a trailing
/// "s" so a question about "my Notions" still finds `Notion`. Deliberately not fuzzy:
/// this decides whether to spend a large, exact block of the prompt, and a matcher
/// that guesses would spend it on the wrong category and crowd out the excerpts that
/// would have answered the question.
pub fn matched_categories(query: &str, census: &Census) -> Vec<String> {
    let q = query.to_lowercase();
    let bytes = q.as_bytes();
    let boundary = |i: usize| !bytes[i].is_ascii_alphanumeric();
    census
        .categories
        .iter()
        .filter(|(cat, _)| {
            let needle = cat.to_lowercase();
            if needle.is_empty() {
                return false;
            }
            q.match_indices(&needle).any(|(at, _)| {
                let before_ok = at == 0 || boundary(at - 1);
                let end = at + needle.len();
                // A trailing plural "s" counts as part of the word, not as the
                // boundary-breaking character that rejects the match.
                let end = if bytes.get(end) == Some(&b's') { end + 1 } else { end };
                before_ok && (end >= bytes.len() || boundary(end))
            })
        })
        .map(|(cat, _)| cat.clone())
        .collect()
}

/// Build the listings for whichever categories the question named, newest first,
/// sharing one cumulative title budget so several matches cannot add up to more
/// than one was allowed.
pub(super) fn inventories(all: &[Memory], query: &str, census: &Census, budget: Budget) -> Vec<Inventory> {
    let wanted = matched_categories(query, census);
    if wanted.is_empty() {
        return vec![];
    }
    let mut left = budget.inventory_titles;
    let mut out = Vec::new();
    for cat in wanted {
        if left == 0 {
            break;
        }
        let mut rows: Vec<&Memory> = all.iter().filter(|m| m.category == cat).collect();
        // Dates are RFC3339, so string ordering is time ordering; newest first, so a
        // list that does hit the cap keeps the half a person is more likely to mean.
        rows.sort_by(|a, b| b.date.cmp(&a.date));
        let total = rows.len();
        let take = total.min(left);
        left -= take;
        out.push(Inventory {
            category: cat,
            titles: rows.into_iter().take(take).map(|m| m.title.clone()).collect(),
            total,
        });
    }
    out
}

/// Everything the four stores returned for one question.
pub struct Recall {
    pub memories: Vec<Memory>,
    pub journal: Vec<JournalHit>,
    pub pages: Vec<Page>,
    /// The profile's own prompt block — already phrased as instructions about the
    /// user, so it is passed through rather than re-rendered here.
    pub profile: String,
    pub census: Census,
    /// Full title listings for the categories the question named. Usually empty —
    /// most questions name no category at all.
    pub inventories: Vec<Inventory>,
    /// Why semantic search fell back to keyword matching, if it did.
    ///
    /// Carried so the caller can SAY it. A brain quietly answering from keyword
    /// matches while everything above it says "semantic" is the failure that hid a
    /// dead embedding model on this machine for months.
    pub degraded: Option<String>,
    /// The budget this was actually gathered under — already reduced by
    /// `Budget::displaced` if an inventory is present.
    ///
    /// Carried on the struct rather than passed to the renderers separately, because
    /// the two must agree: fetching under one budget and rendering under another
    /// silently clips excerpts to a width nothing chose.
    pub budget: Budget,
}
