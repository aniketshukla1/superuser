use super::*;

impl RiskOverrides {
    /// The rules, most specific first, each marked with whether it can bite.
    pub fn view(&self) -> Vec<RuleView> {
        self.rules
            .iter()
            .map(|r| RuleView {
                pattern: r.pattern.clone(),
                risk: label(r.risk).to_string(),
                effective: !floors_a_builtin(&r.pattern, r.risk),
            })
            .collect()
    }

    /// Replace the whole list. Whole-list rather than add/remove because the rules
    /// are order-sensitive through specificity: a client that could only append
    /// would have no way to express "this narrow rule beats that broad one".
    pub fn save(data_dir: &Path, rules: &[(String, String)]) -> Result<Self, String> {
        let mut out: Vec<serde_json::Value> = Vec::new();
        for (pattern, risk) in rules {
            let pattern = pattern.trim();
            if pattern.is_empty() {
                return Err("a rule needs a pattern".into());
            }
            if parse_class(risk).is_none() {
                return Err(format!("{risk:?} is not a risk class — use read, write_local, exec or external"));
            }
            out.push(serde_json::json!({ "pattern": pattern, "risk": risk.trim().to_lowercase() }));
        }
        let path = data_dir.join("risk-overrides.json");
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(Self::load(data_dir))
    }
}

pub(super) fn label(r: RiskClass) -> &'static str {
    match r {
        RiskClass::Read => "read",
        RiskClass::WriteLocal => "write_local",
        RiskClass::Exec => "exec",
        RiskClass::External => "external",
    }
}

/// Would this rule be ignored for naming a built-in and asking for less?
///
/// Checked against every floored name rather than the one the pattern spells,
/// because a glob (`run_*`) reaches them without naming them.
pub(super) fn floors_a_builtin(pattern: &str, want: RiskClass) -> bool {
    FLOORED.iter().any(|name| {
        glob_match(pattern, name)
            && strictness(want) < strictness(base_for_builtin(name))
    })
}

/// What a built-in's class actually is, for the ignored-rule check.
///
/// A small table rather than a lookup into the session's catalog: the catalog is
/// built per-session from a persona's grants, so a tool this session did not grant
/// would read as unknown and the card would call a dropped rule effective.
pub(super) fn base_for_builtin(name: &str) -> RiskClass {
    match name {
        "run_shell" | "computer_use" => RiskClass::Exec,
        "write_file" | "edit_file" | "apply_patch" | "save_skill" | "create_scheduled_task" => RiskClass::WriteLocal,
        _ => RiskClass::External,
    }
}
