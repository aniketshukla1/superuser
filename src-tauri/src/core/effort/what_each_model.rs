//! what each model actually accepts

use super::*;

//
// From the API docs (platform.claude.com/docs/en/build-with-claude/{extended-thinking,
// effort}), read on 2026-09-01 — NOT inferred from the shape of a model id. Version
// arithmetic on a string would have to be re-derived every release and would be wrong
// silently; these lists are wrong loudly, in a test, the day a model is added.
//
// The rule this encodes, in one sentence: `type: "enabled"` + `budget_tokens` is the
// only thinking mode on 4.5 and earlier, is deprecated on 4.6, and returns a 400 on
// 4.7 and later; `type: "adaptive"` is the reverse and 400s on 4.5 and earlier.

/// Models that accept `thinking: {"type": "adaptive"}`. Everything 4.6 and later.
pub(super) const ADAPTIVE: &[&str] = &[
    "claude-opus-4-6", "claude-sonnet-4-6", "claude-opus-4-7", "claude-opus-4-8",
    "claude-opus-5", "claude-sonnet-5", "claude-fable-5", "claude-mythos-5",
    "claude-mythos-preview",
];

/// Models that REJECT `budget_tokens` outright (400). 4.6 still takes it, deprecated.
pub(super) const REJECTS_BUDGET: &[&str] = &[
    "claude-opus-4-7", "claude-opus-4-8", "claude-opus-5", "claude-sonnet-5",
    "claude-fable-5", "claude-mythos-5",
];

/// Models that accept `output_config: {"effort": …}`, and the highest level each one
/// takes. `xhigh` is newer than `max`, so some models that have `max` lack `xhigh` —
/// which is why this is a per-model list of the levels rather than a ceiling.
pub(super) const EFFORT: &[(&str, &[&str])] = &[
    ("claude-fable-5", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-mythos-5", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-opus-5", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-opus-4-8", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-opus-4-7", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-sonnet-5", &["low", "medium", "high", "xhigh", "max"]),
    ("claude-mythos-preview", &["low", "medium", "high", "max"]),
    ("claude-opus-4-6", &["low", "medium", "high", "max"]),
    ("claude-sonnet-4-6", &["low", "medium", "high", "max"]),
    // Opus 4.5 is the one extended-thinking-only model with effort: set both.
    ("claude-opus-4-5", &["low", "medium", "high"]),
];

/// Longest-prefix match, so a dated id (`claude-opus-4-5-20251101`) answers the same
/// as its family and a shorter entry can never shadow a longer one.
pub(super) fn family<'a, T: Copy>(model: &str, table: &[(&'a str, T)]) -> Option<T> {
    let bare = crate::core::providers::model_name(model).to_ascii_lowercase();
    table
        .iter()
        .filter(|(prefix, _)| bare.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, v)| *v)
}

pub(super) fn listed(model: &str, table: &[&str]) -> bool {
    let bare = crate::core::providers::model_name(model).to_ascii_lowercase();
    table.iter().any(|p| bare.starts_with(p))
}

pub fn accepts_adaptive(model: &str) -> bool {
    listed(model, ADAPTIVE)
}

/// Whether `budget_tokens` is safe to send. False only where it is a 400 — 4.6 keeps
/// it, deprecated, and we prefer adaptive there anyway.
pub fn accepts_budget(model: &str) -> bool {
    !listed(model, REJECTS_BUDGET)
}

/// The effort level to actually send: the requested one, or the nearest supported
/// level BELOW it. Stepping down rather than up, because the alternative is charging
/// someone for `max` when they asked for `xhigh` on a model that has one and not the
/// other. `None` means this model has no effort parameter at all.
pub fn effort_for(model: &str, level: &str) -> Option<&'static str> {
    let allowed = family(model, EFFORT)?;
    let want = index_of(level);
    (0..=want).rev().find_map(|i| {
        let candidate = LEVELS[i];
        allowed.contains(&candidate).then_some(candidate)
    })
}

/// The `thinking` and `output_config` fields for one request, for one model.
///
/// The single place that knows which of the two thinking modes a model takes. This
/// existed nowhere: the app sent `budget_tokens` to every Claude model, so the effort
/// slider — its headline "how hard should it think" control — failed the turn with a
/// 400 on every current model the moment it left the left stop. The default hid it,
/// because `low` sends no thinking field at all.
pub fn request_fields(
    model: &str,
    level: &str,
    max_tokens: u32,
) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    let effort = effort_for(model, level)
        .map(|e| serde_json::json!({ "effort": e }));
    let thinking = if accepts_adaptive(model) {
        // Depth is `effort`'s job here; the mode is just "let it decide".
        Some(serde_json::json!({ "type": "adaptive" }))
    } else if accepts_budget(model) {
        thinking_field(level, max_tokens)
    } else {
        // Neither mode: send no thinking field rather than one that 400s.
        None
    };
    (thinking, effort)
}

/// OpenAI-compatible vendors take a name, not a budget. "low" stays off there too:
/// omitting the field is how you ask for no reasoning.
pub fn reasoning_effort(level: &str) -> Option<&'static str> {
    match normalize(level) {
        "low" => None,
        "medium" => Some("medium"),
        "high" => Some("high"),
        // Nothing above "high" exists in that vocabulary; asking for one is a 400.
        "xhigh" | "max" => Some("high"),
        _ => None,
    }
}
