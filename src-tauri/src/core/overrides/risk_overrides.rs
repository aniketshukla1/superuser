use super::*;

/// Built-in tool names whose risk an override may only ever raise.
///
/// Kept as its own list rather than derived from the catalog: the catalog is built
/// per-session from the persona's grants, so a tool absent from THIS session's
/// catalog would otherwise look like an unknown MCP name and become relaxable.
pub(super) const FLOORED: &[&str] = &[
    "run_shell", "send_message", "send_file", "write_file", "edit_file", "apply_patch",
    "computer_use", "spawn_subagent", "connector_sync", "web_fetch", "web_search",
    "browser_open", "request_tool", "save_skill", "create_scheduled_task",
];

/// How sensitive each class is. `RiskClass` is declared least→most sensitive, so
/// this is just its discriminant order made explicit and testable.
pub(super) fn strictness(r: RiskClass) -> u8 {
    match r {
        RiskClass::Read => 0,
        RiskClass::WriteLocal => 1,
        RiskClass::Exec => 2,
        RiskClass::External => 3,
    }
}

pub(super) fn parse_class(s: &str) -> Option<RiskClass> {
    match s.trim().to_lowercase().as_str() {
        "read" => Some(RiskClass::Read),
        "write_local" | "write" => Some(RiskClass::WriteLocal),
        "exec" => Some(RiskClass::Exec),
        "external" => Some(RiskClass::External),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct RawRule {
    pub(super) pattern: String,
    pub(super) risk: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Rule {
    pub(super) pattern: String,
    pub(super) risk: RiskClass,
}

/// Does `name` match a glob of `*` (any run, including empty) and `?` (one char)?
///
/// Written out rather than translated to a regex because a translation has to
/// escape every other metacharacter correctly, and a missed one turns a user's
/// literal `.` into "any character" — quietly widening the rule they wrote.
pub(super) fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    // Classic two-pointer wildcard match with backtracking on the last `*`.
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            mark = ni;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// More literal characters = more specific. An exact pattern beats every glob.
pub(super) fn specificity(pattern: &str) -> usize {
    let literal = pattern.chars().filter(|c| !matches!(c, '*' | '?')).count();
    if pattern.contains('*') || pattern.contains('?') { literal } else { literal + 1000 }
}

/// The user's overrides, read from `<data_dir>/risk-overrides.json`.
///
/// ```json
/// [ { "pattern": "notion_*", "risk": "read" },
///   { "pattern": "notion_delete_*", "risk": "external" } ]
/// ```
#[derive(Debug, Default, Clone)]
pub struct RiskOverrides {
    pub(super) rules: Vec<Rule>,
}

impl RiskOverrides {
    /// Load, tolerating everything. A malformed file means no overrides, never a
    /// failed tool call: this is a convenience layer, and the base classification
    /// underneath it is always safe on its own.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("risk-overrides.json");
        let Ok(text) = std::fs::read_to_string(&path) else { return Self::default() };
        let Ok(raw) = serde_json::from_str::<Vec<RawRule>>(&text) else { return Self::default() };
        let mut rules: Vec<Rule> = raw
            .into_iter()
            .filter(|r| !r.pattern.trim().is_empty())
            .filter_map(|r| parse_class(&r.risk).map(|risk| Rule { pattern: r.pattern.trim().to_string(), risk }))
            .collect();
        // Most specific first, so `resolve` can take the first match.
        rules.sort_by_key(|r| std::cmp::Reverse(specificity(&r.pattern)));
        Self { rules }
    }

    #[cfg(test)]
    pub(super) fn from_pairs(pairs: &[(&str, RiskClass)]) -> Self {
        let mut rules: Vec<Rule> =
            pairs.iter().map(|(p, r)| Rule { pattern: p.to_string(), risk: *r }).collect();
        rules.sort_by_key(|r| std::cmp::Reverse(specificity(&r.pattern)));
        Self { rules }
    }

    /// The most specific rule matching this tool, if any.
    pub(super) fn resolve(&self, tool: &str) -> Option<RiskClass> {
        self.rules.iter().find(|r| glob_match(&r.pattern, tool)).map(|r| r.risk)
    }

    /// The risk class actually used for this call.
    ///
    /// An override on a built-in may only tighten; on anything else it may do
    /// either. A loosening rule on a built-in is not an error — it is ignored, and
    /// the base stands. Erroring would be worse: one bad line would break every
    /// tool call rather than the one rule.
    pub fn effective(&self, tool: &str, base: RiskClass) -> RiskClass {
        match self.resolve(tool) {
            None => base,
            Some(want) => {
                if FLOORED.contains(&tool) && strictness(want) < strictness(base) {
                    base
                } else {
                    want
                }
            }
        }
    }
}

/// One rule as the settings card shows it.
#[derive(Debug, Clone, Serialize)]
pub struct RuleView {
    pub pattern: String,
    pub risk: String,
    /// False when this rule can never take effect: it names a built-in tool and
    /// asks for something looser than the built-in's own class.
    ///
    /// Reported rather than rejected on save, and rendered rather than hidden. A
    /// rule that is silently dropped is how someone concludes the feature is
    /// broken; a rule shown as "ignored — cannot loosen a built-in" teaches the
    /// boundary in one line.
    pub effective: bool,
}
