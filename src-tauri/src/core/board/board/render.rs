use super::*;

impl Board {
    /// The board as the model should read it: grouped, with what is blocking each.
    pub fn render(&self, filter: Option<ItemState>) -> String {
        let items = self.items();
        let shown: Vec<&Item> = items.iter().filter(|i| filter.map_or(true, |f| i.state == f)).collect();
        if shown.is_empty() {
            return match filter {
                Some(f) => format!("Nothing on the board is {}.", f.label()),
                None => "The board is empty.".into(),
            };
        }
        let mut out = String::new();
        for state in [
            ItemState::Review,
            ItemState::InProgress,
            ItemState::Blocked,
            ItemState::Open,
            ItemState::Done,
            ItemState::Canceled,
        ] {
            let group: Vec<&&Item> = shown.iter().filter(|i| i.state == state).collect();
            if group.is_empty() {
                continue;
            }
            out.push_str(&format!("\n## {}\n", state.label()));
            for i in group {
                out.push_str(&format!("- `{}` {}", i.id, i.title));
                if let Some(a) = &i.assignee {
                    out.push_str(&format!(" — {a}"));
                }
                if state == ItemState::Review {
                    match &i.builder {
                        Some(b) => out.push_str(&format!(" (built by {b}; needs a different reviewer)")),
                        None => {}
                    }
                }
                if !i.acceptance.is_empty() {
                    out.push_str(&format!("\n  done when: {}", i.acceptance));
                }
                for r in &i.reviews {
                    out.push_str(&format!("\n  review by {}: {} — {}", r.by, if r.pass { "pass" } else { "fail" }, r.evidence));
                }
                out.push('\n');
            }
        }
        out
    }
}
