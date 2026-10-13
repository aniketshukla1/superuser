
/// Faster → smarter. The order is load-bearing: the slider renders it directly and
/// `index_of`/`at` convert between the name and the slider position.
pub const LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

pub const DEFAULT: &str = "high";

/// The API's own floor. Below this a thinking block cannot be requested at all.
pub(super) const MIN_BUDGET: u32 = 1024;

pub fn is_level(s: &str) -> bool {
    LEVELS.contains(&s)
}

/// Normalise anything that arrives from settings, a URL or an older config.
pub fn normalize(s: &str) -> &'static str {
    let s = s.trim().to_ascii_lowercase();
    LEVELS.iter().copied().find(|l| *l == s).unwrap_or(DEFAULT)
}

pub fn index_of(level: &str) -> usize {
    LEVELS.iter().position(|l| *l == normalize(level)).unwrap_or(2)
}

pub fn at(index: usize) -> &'static str {
    LEVELS.get(index).copied().unwrap_or(DEFAULT)
}

/// The extended-thinking budget for a level, or None for "do not ask it to think".
///
/// Two hard constraints from the API, and both are the reason this is a function
/// rather than a table: the budget must be at least 1024, and it must be strictly
/// less than `max_tokens` — the thinking comes out of the same reply. A budget
/// derived as a percentage of a small `max_tokens` violates both, so a request built
/// from a naive table would 400 for anyone who had turned their reply length down.
/// When there is genuinely no room to think, the honest answer is None: send a
/// normal request rather than an invalid one.
pub fn budget(level: &str, max_tokens: u32) -> Option<u32> {
    let pct: u32 = match normalize(level) {
        // The fast end is not "think a little" — it is "do not think", which is what
        // makes it fast. The slider's left label says exactly that.
        "low" => return None,
        "medium" => 25,
        "high" => 50,
        "xhigh" => 75,
        "max" => 90,
        _ => return None,
    };
    if max_tokens <= MIN_BUDGET {
        return None;
    }
    let want = (u64::from(max_tokens) * u64::from(pct) / 100) as u32;
    Some(want.clamp(MIN_BUDGET, max_tokens - 1))
}

/// The `thinking` field for a request body, ready to merge in — or None.
///
/// MANUAL mode only. Correct on Claude 4.5 and earlier, deprecated on 4.6, and a 400
/// on 4.7 and later — so it must not be reached without asking [`accepts_budget`]
/// first. `request_fields` is the entry point that gets this right; this stays public
/// for the models that still take it.
pub fn thinking_field(level: &str, max_tokens: u32) -> Option<serde_json::Value> {
    budget(level, max_tokens).map(|b| {
        serde_json::json!({ "type": "enabled", "budget_tokens": b })
    })
}
