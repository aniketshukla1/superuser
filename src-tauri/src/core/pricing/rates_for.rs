use super::*;

/// Dollars per million tokens, by billing class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
    /// Usually `input × 1.25` — the premium for writing the cache.
    pub cache_write: f64,
    /// Usually `input × 0.1` — the reason any of this is worth doing.
    pub cache_read: f64,
}

impl Rates {
    /// The standard Anthropic shape: writes at 1.25×, reads at 0.1×. Stated once so
    /// a new model is two numbers rather than four, and the multipliers cannot drift
    /// apart between entries.
    pub(super) const fn anthropic(input: f64, output: f64) -> Rates {
        Rates { input, output, cache_write: input * 1.25, cache_read: input * 0.1 }
    }

    pub fn cost(&self, u: &TokenUsage) -> f64 {
        const M: f64 = 1_000_000.0;
        (u.input as f64 * self.input
            + u.output as f64 * self.output
            + u.cache_write as f64 * self.cache_write
            + u.cache_read as f64 * self.cache_read)
            / M
    }
}

/// Free, and known to be free. Everything here runs on the user's own hardware.
pub(super) const FREE: Rates = Rates { input: 0.0, output: 0.0, cache_write: 0.0, cache_read: 0.0 };

/// Rates per model family, matched on a prefix of the id.
///
/// Prefix rather than exact id: the registry ships both dated and undated ids
/// (`claude-haiku-4-5-20251001` alongside `claude-sonnet-5`), and a table keyed on
/// exact strings would silently fall through to "unknown" the day a date suffix
/// appears. Longest prefix wins, so `claude-opus-5` cannot be shadowed by a shorter
/// `claude-opus` entry added later.
pub(super) const TABLE: &[(&str, Rates)] = &[
    ("claude-opus-5", Rates::anthropic(5.0, 25.0)),
    ("claude-opus-4-8", Rates::anthropic(5.0, 25.0)),
    ("claude-opus-4-7", Rates::anthropic(15.0, 75.0)),
    ("claude-sonnet-5", Rates::anthropic(3.0, 15.0)),
    ("claude-sonnet-4", Rates::anthropic(3.0, 15.0)),
    ("claude-fable-5", Rates::anthropic(5.0, 25.0)),
    ("claude-haiku-4-5", Rates::anthropic(1.0, 5.0)),
    ("claude-3-5-haiku", Rates::anthropic(0.8, 4.0)),
];

/// The rates for a model id, or `None` if we genuinely do not know.
///
/// "Is this local?" is answered by [`crate::core::providers`] rather than by a list
/// of prefixes here, because that registry already gets right the case a prefix list
/// gets wrong: an Ollama tag is `name:tag`, which looks exactly like
/// `provider:model`. `gemma4:12b` has no provider prefix at all and is nonetheless
/// running on this machine — `providers::ollama_tags_are_not_provider_prefixes`
/// documents the same trap, which a second copy of the rule here would have to
/// re-learn every time the registry changed.
pub fn rates_for(model: &str) -> Option<Rates> {
    use crate::core::providers;
    let id = model.trim();
    if id.is_empty() {
        return None;
    }
    // Local means free, and "is it local" is `providers`' question to answer — see
    // `runs_locally` for why a prefix test here would get `gemma4:12b` wrong.
    if providers::runs_locally(id) {
        return Some(FREE);
    }
    match providers::resolve(id) {
        // OpenCode Zen names its free tier with a `-free` suffix, and the registry
        // curates exactly those ids. Taking the provider at its word beats reporting
        // "unknown cost" for a model whose name says what it costs — and this is the
        // default model here, so the alternative is a permanently blank dashboard.
        Some(p) if p.id == "opencode" && providers::model_name(id).ends_with("-free") => Some(FREE),
        // A cloud provider: price the bare model name, without the routing prefix.
        Some(_) => lookup(providers::model_name(id)),
        // Unreachable: `runs_locally` already returned true for the unresolved case.
        None => Some(FREE),
    }
}

/// Longest matching prefix in the rate table.
pub(super) fn lookup(bare: &str) -> Option<Rates> {
    let bare = bare.trim().to_ascii_lowercase();
    TABLE
        .iter()
        .filter(|(prefix, _)| bare.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, r)| *r)
}

/// What this usage cost on this model. `None` means "no rate for that id" — the
/// caller must render that as unknown, never as zero.
pub fn cost_of(model: &str, usage: &TokenUsage) -> Option<f64> {
    rates_for(model).map(|r| r.cost(usage))
}
