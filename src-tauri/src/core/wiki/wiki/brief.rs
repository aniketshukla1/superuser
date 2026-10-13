use super::*;

impl Wiki {
    /// What the reviewer is shown: the patterns it already knows and the changes it
    /// has already tried. Bounded, because it rides in a single prompt alongside the
    /// transcript and the skills.
    ///
    /// Ordering is load-bearing. Patterns come most-reinforced first, so a truncated
    /// brief loses the one-off observations rather than the recurring ones.
    pub fn brief(&self, max_patterns: usize) -> String {
        let ps = self.patterns();
        let ims = self.impacts();
        if ps.is_empty() && ims.is_empty() {
            return String::new();
        }
        let mut s = String::from("## What earlier reviews already learned\n\n");
        if ps.is_empty() {
            s.push_str("(nothing yet)\n");
        } else {
            s.push_str(
                "Patterns from past conversations. If this conversation shows one of these again, \
                 reinforce it by writing a pattern with the SAME id — do not invent a second id for \
                 the same thing. A pattern seen several times is a strong reason to fix a skill.\n\n",
            );
            for p in ps.iter().take(max_patterns) {
                s.push_str(&format!(
                    "- `{}` ({}, seen {}×) — {}\n",
                    p.id,
                    p.kind.as_str(),
                    p.seen,
                    p.title.trim()
                ));
            }
            if ps.len() > max_patterns {
                s.push_str(&format!("- …and {} less-repeated patterns\n", ps.len() - max_patterns));
            }
        }

        let undone: Vec<&Impact> = ims.iter().filter(|i| i.verdict == Verdict::Undone).collect();
        if !undone.is_empty() {
            s.push_str("\n## Changes the user has already rejected\n\nThese were written and then undone. Do NOT propose them again.\n\n");
            for i in undone.iter().rev().take(12) {
                s.push_str(&format!("- `{}` — {}\n", i.target, if i.note.is_empty() { "(no note)" } else { i.note.trim() }));
            }
        }
        s
    }
}
