use super::*;

impl Loaded {
    /// Decide one action. Never panics, and never answers without a reason.
    pub fn decide(&self, ctx: &Value) -> Decision {
        let mode = self.boundaries.mode.as_str().to_string();
        let record = self.boundaries.mode == Mode::Record;
        for (rule, compiled) in self.boundaries.deny.iter().zip(&self.compiled_deny) {
            let verdict = match compiled {
                Ok(p) => p.test(ctx),
                Err(e) => Err(e.clone()),
            };
            match verdict {
                Ok(false) => continue,
                Ok(true) => {
                    return Decision {
                        allowed: false,
                        mode,
                        source: "deny".into(),
                        rule: rule.when.clone(),
                        rule_name: rule.name.clone(),
                        carried_out: record,
                        reason: String::new(),
                    }
                }
                // Broken, so it denies. Said as broken, so nobody reads the refusal as
                // the rule having matched.
                Err(e) => {
                    return Decision {
                        allowed: false,
                        mode,
                        source: "deny".into(),
                        rule: rule.when.clone(),
                        rule_name: rule.name.clone(),
                        carried_out: record,
                        reason: match &self.problem {
                            Some(p) => p.clone(),
                            None => format!("the rule can't be evaluated: {e}"),
                        },
                    }
                }
            }
        }
        for (expr, compiled) in self.boundaries.allow.iter().zip(&self.compiled_allow) {
            if let Ok(p) = compiled {
                if p.test(ctx) == Ok(true) {
                    return Decision {
                        allowed: true,
                        mode,
                        source: "allow".into(),
                        rule: expr.clone(),
                        carried_out: true,
                        ..Default::default()
                    };
                }
            }
        }
        Decision {
            allowed: false,
            mode,
            source: "default".into(),
            carried_out: record,
            reason: "nothing on the allow list permits it".into(),
            ..Default::default()
        }
    }
}

/// Every problem with a set of boundaries, before they are saved.
pub fn check(b: &Boundaries) -> Vec<RuleProblem> {
    let mut out = Vec::new();
    for (i, r) in b.deny.iter().enumerate() {
        if let Some(message) = check_rule(&r.when) {
            out.push(RuleProblem { list: "deny".into(), index: i, message });
        }
    }
    for (i, a) in b.allow.iter().enumerate() {
        if let Some(message) = check_rule(a) {
            out.push(RuleProblem { list: "allow".into(), index: i, message });
        }
    }
    out
}

/// What is wrong with one rule, or `None`. Reads the expression, checks every field
/// it names exists (suggesting the nearest one), and runs it once against an action
/// where nothing applies, which catches a rule that answers text or a number.
pub fn check_rule(expr: &str) -> Option<String> {
    let program = match rules::parse(expr) {
        Ok(p) => p,
        Err(e) => return Some(format!("This rule can't be read {e}.")),
    };
    for (field, _) in program.fields() {
        let known = FIELDS.iter().any(|(f, _)| *f == field)
            // A group on its own (`page`, `element`) is a real value, just not one a
            // comparison would usually want. Allowed; the neutral run below will say
            // if it answers the wrong kind of thing.
            || FIELDS.iter().any(|(f, _)| f.starts_with(&format!("{field}.")));
        if !known {
            let near = FIELDS
                .iter()
                .map(|(f, _)| (distance(f, &field), *f))
                .min_by_key(|(d, _)| *d)
                .filter(|(d, _)| *d <= 3)
                .map(|(_, f)| format!(" — did you mean `{f}`?"))
                .unwrap_or_default();
            return Some(format!("There's no field called `{field}`{near}"));
        }
    }
    let neutral = context(&Subject::default(), "", "", &Initiator::new("person", ""));
    match program.test(&neutral) {
        Ok(_) => None,
        Err(e) => Some(format!("This rule can't be answered: {e}.")),
    }
}

/// Edit distance, for "did you mean". Fields are short; this is never slow.
pub(super) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let sub = prev[j - 1] + usize::from(a[i - 1] != b[j - 1]);
            cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}
