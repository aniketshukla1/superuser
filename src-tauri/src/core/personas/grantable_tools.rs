use super::*;

/// Tool capability ids a persona may declare. Validated at parse time so a bad
/// manifest fails loudly rather than producing a broken persona. Grows as
/// `core/agent` gains tools.
pub const KNOWN_TOOLS: &[&str] = &[
    "files", "search", "shell", "git", "todo", "plan", "ask", "subagent", "web",
    "recall_memory", "computer", "connectors", "messaging", "schedule", "knowledge",
    "toolchain", "board", "browser", "authoring", "expenses", "activity", "metrics", "talent",
    // Always on, like `plan` and `ask`, so it is knowable but not grantable — a
    // persona cannot be created without it and there is nothing to pick.
    "recall",
];

/// What an assistant can be GRANTED when one is created.
///
/// This was six ids while the catalogue had fifteen, so nine capabilities could not
/// be given to an assistant the user made — including `knowledge` and `schedule`,
/// which the built-in assistant has, and `messaging`, which nothing had. A picker
/// that silently drops what you choose is worse than one that does not offer it.
///
/// `ALWAYS_ON` is not here: those are given to everyone, so there is nothing to
/// pick. The risky ones stay in the list rather than being hidden — granting them
/// is the user's decision, and the permission MODE still governs what they may
/// actually do.
pub const GRANTABLE_TOOLS: &[&str] = &[
    "files", "search", "shell", "git", "todo", "web", "recall_memory",
    "connectors", "messaging", "subagent", "schedule", "knowledge", "computer",
    "toolchain", "board", "browser", "authoring", "expenses", "activity", "metrics", "talent",
];

pub(super) const VALID_FAMILIES: &[&str] = &["code", "knowledge"];

pub(super) const VALID_MODES: &[&str] = &["discuss", "plan", "interactive", "custom", "auto"];

pub(super) const VALID_TIERS: &[&str] = &["core", "optional"];

/// The persona a fresh install starts on (and falls back to).
pub const DEFAULT_PERSONA_ID: &str = "assistant";

pub(super) fn id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-z0-9][a-z0-9_-]{0,63}$").unwrap())
}
