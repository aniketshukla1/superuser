use super::*;

/// How many internal memories may be inlined. Small: this is a head start, not a
/// context dump, and everything here is paid for on every first turn.
pub(super) const INLINE_LIMIT: usize = 4;

/// Characters of body per inlined memory.
pub(super) const BODY_CHARS: usize = 400;

/// How many standing rules ride along regardless of what was asked.
pub(super) const STANDING_LIMIT: usize = 3;

/// Categories whose memories are *always* in play, however the question is worded.
///
/// The gap this closes, found by watching a scoreboard task fail for the wrong
/// reason. Everything else in this brief is keyword-matched against the user's
/// opening message, which is right for facts — "what do we know about the deploy
/// pipeline" should not drag in a note about someone's birthday. It is exactly
/// wrong for standing rules. "Always answer in three bullets" shares no words with
/// "what are the risks of deploying on a Friday", scores zero, and never arrives —
/// so a preference the user deliberately wrote down applies only to questions that
/// happen to be *about* preferences.
///
/// A rule is not on-topic. It is the frame the topic is answered in.
pub(super) const STANDING_CATEGORIES: &[&str] = &["preferences", "preference", "rules", "instructions"];

pub(super) fn is_standing(m: &crate::core::memory::Memory) -> bool {
    let c = m.category.trim().to_lowercase();
    STANDING_CATEGORIES.contains(&c.as_str())
        || m.tags.iter().any(|t| {
            let t = t.trim().to_lowercase();
            t == "preference" || t == "preferences" || t == "standing-rule"
        })
}

/// How wide to cast the net before splitting by origin.
pub(super) const SCAN: usize = 24;

/// What the sweep found, kept separate from its rendering so the decision about
/// what may be shown is testable on its own.
#[derive(Debug, Default, Clone)]
pub struct Brief {
    /// Standing rules the user wrote, carried whatever the question was.
    pub standing: Vec<Inlined>,
    /// Internal-origin memories, safe to put in front of the model verbatim.
    pub inlined: Vec<Inlined>,
    /// External matches, counted by source. Never their content.
    pub withheld: Vec<Withheld>,
}

#[derive(Debug, Clone)]
pub struct Inlined {
    pub title: String,
    pub summary: String,
    pub body: String,
    pub date: String,
}

/// The name used for a file that carries no connector prefix.
///
/// Not "your vault": *where a file is* isn't *where it came from*, and these are
/// withheld precisely because their origin can't be established. Saying "yours"
/// about something we are refusing to show you on the grounds that it might not
/// be yours would be the page contradicting itself.
pub(super) const UNATTRIBUTED: &str = "no recorded source";

/// A source that has matching material the model has not been shown.
#[derive(Debug, Clone)]
pub struct Withheld {
    /// The connector prefix off the filename — `github`, `gmail`. Ours, not theirs.
    pub source: String,
    pub count: usize,
    /// Oldest and newest dates among the matches, when they carry one.
    pub from: String,
    pub to: String,
}

impl Brief {
    pub fn is_empty(&self) -> bool {
        self.standing.is_empty() && self.inlined.is_empty() && self.withheld.is_empty()
    }
}

/// The connector a vault file came from. Filenames are `<connector>-<id>.md` by
/// convention across every integration, the same attribution the ledger uses.
///
/// Checked against the connectors we actually have rather than trusting any
/// prefix, because a prefix is not a source: `user-profile.md` would otherwise be
/// reported as "1 item from user", which names a connector that doesn't exist.
/// Anything unrecognised is reported as having no recorded source.
pub(super) fn source_of(filename: &str) -> String {
    const BUILT_IN: &[&str] = &["github", "notion", "gmail", "calendar", "linear", "stripe", "slack", "email", "whatsapp"];
    match filename.split_once('-') {
        Some((prefix, _))
            if BUILT_IN.contains(&prefix) || crate::integrations::rest::find_spec(prefix).is_some() =>
        {
            prefix.to_string()
        }
        _ => UNATTRIBUTED.to_string(),
    }
}

/// `2026-07-25T20:09:59.607627+00:00` → `2026-07-25`.
pub(super) fn day_of(date: &str) -> String {
    date.split('T').next().unwrap_or("").trim().to_string()
}

/// Sweep memory for anything bearing on the opening message.
///
/// Free: `MemoryEngine::recall` is keyword scoring over files already on disk —
/// no model call, no embedding, no network. That is what makes it affordable to
/// run on every new thread rather than behind a toggle nobody finds.
pub fn sweep(memory: &MemoryEngine, message: &str) -> Brief {
    // `recall_scored_split` rather than `recall` + `list`: the latter reads and
    // parses every file in the vault twice, which doubled the latency in front of
    // the first token of every new conversation. Measured at 5,000 memories: 266 ms
    // became 132 ms for the same answer.
    //
    // *Split*, and not a single top-24, because the two tiers below are not
    // competing for the same slots. Ranking them in one pool let the external
    // majority take all 24 places on any vault big enough to matter, and the
    // inlined tier — the only one that carries content — came back empty at 1,000
    // memories and empty at 5,000. See `recall_scored_split` for the numbers.
    let (internal, external) = memory.recall_scored_split(message, INLINE_LIMIT, SCAN);

    // Standing rules first, and outside the ranking entirely — see
    // `STANDING_CATEGORIES`. Internal only, so this adds no trust surface: it is
    // the user's own words either way, and the only thing that changes is that a
    // rule no longer has to be topically relevant to its own application.
    let mut standing: Vec<crate::core::memory::Memory> =
        memory.list().into_iter().filter(|m| m.origin == Origin::Internal && is_standing(m)).collect();
    standing.sort_by(|a, b| b.date.cmp(&a.date));
    standing.truncate(STANDING_LIMIT);

    if internal.is_empty() && external.is_empty() && standing.is_empty() {
        return Brief::default();
    }
    let mut brief = Brief::default();
    let mut by_source: BTreeMap<String, (usize, String, String)> = BTreeMap::new();

    for m in &standing {
        brief.standing.push(Inlined {
            title: m.title.clone(),
            summary: m.summary.clone(),
            body: m.body.chars().take(BODY_CHARS).collect(),
            date: day_of(&m.date),
        });
    }

    // The user's own words, or something written on this machine. Nothing outside
    // wrote them, so nothing outside can steer the run through them.
    for (m, _score) in &internal {
        // Already above, and paying for it twice would crowd out a third fact.
        if standing.iter().any(|s| s.filename == m.filename) {
            continue;
        }
        brief.inlined.push(Inlined {
            title: m.title.clone(),
            summary: m.summary.clone(),
            body: m.body.chars().take(BODY_CHARS).collect(),
            date: day_of(&m.date),
        });
    }

    // Everything else is counted, never quoted.
    for (m, _score) in &external {
        let e = by_source.entry(source_of(&m.filename)).or_insert((0, String::new(), String::new()));
        e.0 += 1;
        // Vault dates are full RFC3339 stamps. A brief wants the day: the
        // microseconds cost tokens on every first turn and tell nobody
        // anything. (Only visible against a real vault — the fixtures
        // here all happened to use bare dates.)
        let day = day_of(&m.date);
        if !day.is_empty() {
            if e.1.is_empty() || day < e.1 {
                e.1 = day.clone();
            }
            if day > e.2 {
                e.2 = day;
            }
        }
    }

    brief.withheld = by_source
        .into_iter()
        .map(|(source, (count, from, to))| Withheld { source, count, from, to })
        .collect();
    // Most material first: which source has the most to say is itself a hint.
    brief.withheld.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.source.cmp(&b.source)));
    brief
}
