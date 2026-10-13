use super::super::*;

/// Turn the optional history into the messages this model should replay.
///
/// A turn whose stored traffic is in this model's own wire shape comes back whole —
/// tool calls and results intact, so nothing is read or run twice. Anything else is
/// projected to labelled text, which is what keeps switching models mid-session
/// working. See `core::transcript`.
pub(in crate::commands) fn hist_json(history: &Option<Vec<HistMsg>>, profile: &crate::core::model_profile::ModelProfile) -> Vec<serde_json::Value> {
    let Some(h) = history else { return Vec::new() };
    let triples: Vec<(String, String, Option<crate::core::transcript::TurnWire>)> =
        h.iter().map(|m| (m.role.clone(), m.content.clone(), m.wire.clone())).collect();
    let out = crate::core::transcript::replay(&triples, profile);
    // The one line that says whether H2 is actually fixed for this turn. "The model
    // re-read the file" and "its earlier tool calls reached it as text" are the same
    // symptom from outside, and only this tells them apart.
    let (native, projected) = triples.iter().fold((0, 0), |(n, p), (_, _, w)| match w {
        Some(w) if w.replays_natively_into(profile) => (n + 1, p),
        Some(_) => (n, p + 1),
        None => (n, p),
    });
    crate::trace!(
        "turn",
        "history {} entries -> {} messages ({} native, {} projected, {} plain)",
        triples.len(),
        out.len(),
        native,
        projected,
        triples.len() - native - projected
    );
    out
}

/// Trim a model's code output down to raw insertable code: unwrap a fenced block
/// if present and cap the length so ghost text / inline edits stay reasonable.
pub(in crate::commands) fn clean_completion(s: &str) -> String {
    let mut t = s.trim_matches('\n').to_string();
    if t.starts_with("```") {
        t = t.splitn(2, '\n').nth(1).unwrap_or("").to_string();
        if let Some(i) = t.rfind("```") { t.truncate(i); }
    }
    t.trim_end().to_string()
}
