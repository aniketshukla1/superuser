use super::*;

impl LoopShape {
    /// Split by relevance to the turn instead of by a fixed priority order.
    ///
    /// HARNESS-RESEARCH-2026 §4. Four 2026 papers measured accuracy peaking at 5–15
    /// tools and degrading past it; we measured the opposite — narrowing
    /// `llama3.2:3b` to 13 tools scored 6/19 against 10/19 with the full catalogue,
    /// stable across four ablations. Both can be true, and the reconciliation names
    /// the arm nobody ran: every one of those papers selects **by relevance to the
    /// current query**, and we selected by a fixed order anchored on `RESIDENT`. A
    /// static subset is a different catalogue, not a smaller one, which is exactly
    /// the brittleness `loopshape`'s own comment observed and could not explain.
    ///
    /// Pure, and takes scores rather than computing them: the sharp edge is which
    /// tools get sent, and nothing that needs a network belongs in the thing under
    /// test. `relevance` is name → score; a tool missing from it ranks last, which
    /// is what happens when the embedder is unavailable and the map is empty. That
    /// case degrades to `ALWAYS_SENT` plus arbitrary fill rather than to an empty
    /// wire.
    pub fn split_tools_retrieved(
        &self,
        specs: &[ToolSpec],
        relevance: &std::collections::BTreeMap<String, f32>,
    ) -> (Vec<ToolSpec>, Vec<ToolSpec>) {
        let Some(budget) = self.tool_budget else {
            return (specs.to_vec(), Vec::new());
        };
        let always = |t: &ToolSpec| Self::ALWAYS_SENT.contains(&t.name.as_str());
        let mut ranked: Vec<&ToolSpec> = specs.iter().collect();
        ranked.sort_by(|a, b| {
            always(b)
                .cmp(&always(a))
                .then_with(|| {
                    let sa = relevance.get(&a.name).copied().unwrap_or(f32::MIN);
                    let sb = relevance.get(&b.name).copied().unwrap_or(f32::MIN);
                    sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                })
                // Ties break on the catalogue's own order so a run is reproducible.
                .then_with(|| {
                    let ia = specs.iter().position(|s| s.name == a.name).unwrap_or(usize::MAX);
                    let ib = specs.iter().position(|s| s.name == b.name).unwrap_or(usize::MAX);
                    ia.cmp(&ib)
                })
        });
        let mut sent = Vec::new();
        let mut held = Vec::new();
        let mut spent = 0;
        for t in ranked {
            let cost = schema_tokens(t);
            // Same rule as the static split: stop at the first tool that does not
            // fit rather than filling the gap with whatever happens to be small.
            if !held.is_empty() || spent + cost > budget {
                held.push(t.clone());
            } else {
                spent += cost;
                sent.push(t.clone());
            }
        }
        let order = |v: &mut Vec<ToolSpec>| {
            v.sort_by_key(|t| specs.iter().position(|s| s.name == t.name).unwrap_or(usize::MAX))
        };
        order(&mut sent);
        order(&mut held);
        (sent, held)
    }

    pub fn split_tools(&self, specs: &[ToolSpec]) -> (Vec<ToolSpec>, Vec<ToolSpec>) {
        let Some(budget) = self.tool_budget else {
            return (specs.to_vec(), Vec::new());
        };
        let mut ranked: Vec<&ToolSpec> = specs.iter().collect();
        ranked.sort_by_key(|t| !agent::is_resident(&t.name));
        let mut sent = Vec::new();
        let mut held = Vec::new();
        let mut spent = 0;
        for t in ranked {
            let cost = schema_tokens(t);
            if !held.is_empty() || spent + cost > budget {
                held.push(t.clone());
            } else {
                spent += cost;
                sent.push(t.clone());
            }
        }
        // Restore the caller's order, so nothing downstream depends on the rank the
        // budget was computed in.
        let order = |v: &mut Vec<ToolSpec>| {
            v.sort_by_key(|t| specs.iter().position(|s| s.name == t.name).unwrap_or(usize::MAX))
        };
        order(&mut sent);
        order(&mut held);
        (sent, held)
    }
}
