//! The handoff group's checks

use super::*;

/// Who a turn asked, in the order it asked them.
pub(super) fn coworkers_asked(r: &TurnRecord) -> Vec<String> {
    r.calls
        .iter()
        .filter(|c| c.name == "ask_coworker")
        .map(|c| {
            serde_json::from_str::<serde_json::Value>(&c.args)
                .ok()
                .and_then(|v| v["coworker"].as_str().map(str::to_string))
                .unwrap_or_default()
        })
        .collect()
}

/// It asked the assistant that holds the fact, and the answer it gives carries what came
/// back. The marker lives in that assistant's standing role and nowhere else on the
/// machine, so an answer holding it was relayed and not guessed.
pub(super) fn asked_and_used_the_answer(r: &TurnRecord) -> Result<(), String> {
    if !r.called("ask_coworker") {
        let names: Vec<&str> = r.calls.iter().map(|c| c.name.as_str()).collect();
        return Err(format!("never asked the assistant that keeps it; called {names:?}"));
    }
    r.said_marker()
        .then_some(())
        .ok_or_else(|| format!("asked, and the answer doesn't carry what came back: {:?}", r.answer.trim()))
}

/// With two to choose from, it asked the one whose job this is — and if it asked the other
/// first, it did not stop there and pass "they don't know" back as the answer.
pub(super) fn asked_the_one_that_knows(r: &TurnRecord) -> Result<(), String> {
    let asked = coworkers_asked(r);
    if asked.is_empty() {
        let names: Vec<&str> = r.calls.iter().map(|c| c.name.as_str()).collect();
        return Err(format!("never asked anybody; called {names:?}"));
    }
    if !asked.iter().any(|a| a.to_lowercase().contains("badge")) {
        return Err(format!("asked {asked:?}, and neither of those keeps door codes"));
    }
    r.said_marker()
        .then_some(())
        .ok_or_else(|| format!("asked, and the answer doesn't carry the code: {:?}", r.answer.trim()))
}
