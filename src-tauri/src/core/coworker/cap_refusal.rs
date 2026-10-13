use super::*;

/// The caps, checked before anything is sent. Both say the same thing from two
/// directions — this run has gone as far as delegating goes — and both leave the model
/// somewhere to go: answer with what it has, or ask the person.
pub fn cap_refusal(depth: u8, asked_this_turn: usize) -> Option<String> {
    if depth >= MAX_DEPTH {
        return Some(format!(
            "This work is already {depth} assistants away from the person, which is as far as it goes. \
             Answer with what you have, or say what is still needed."
        ));
    }
    if asked_this_turn >= MAX_PER_TURN {
        return Some(format!(
            "You've asked {asked_this_turn} assistants this turn, which is as many as one turn may. \
             Answer with what you have, or say what is still needed."
        ));
    }
    None
}

pub fn duplicate_refusal(name: &str) -> String {
    format!("You've already asked {name} exactly this. Use the answer you got rather than asking again.")
}

/// The answer, on its way back to the run that asked.
pub fn relayed(name: &str, answer: &str) -> String {
    let answer = answer.trim();
    if answer.is_empty() {
        return format!("{name} finished without saying anything. Say that plainly; don't answer on its behalf.");
    }
    format!(
        "{name} answered:\n\n{answer}\n\nUse this in your own answer and say it came from {name}. \
         If it isn't enough, say what is missing rather than asking again."
    )
}

pub fn timed_out(name: &str) -> String {
    format!(
        "{name} didn't finish within five minutes, so there is no answer. Say that plainly, \
         don't answer on its behalf, and don't ask it again this turn."
    )
}

pub fn failed(name: &str, why: &str) -> String {
    format!("{name} couldn't run: {why}. Say that plainly; don't answer on its behalf.")
}

/// The stricter of two permission modes, so a hop can never act more freely than the run
/// that started it. Anything unknown is treated as the strictest thing it could be.
pub fn stricter(a: &str, b: &str) -> String {
    let rank = |m: &str| match m {
        "discuss" => 0,
        "plan" => 1,
        "interactive" => 2,
        "custom" => 2,
        "auto" => 3,
        _ => 0,
    };
    if rank(a) <= rank(b) { a.to_string() } else { b.to_string() }
}

/// The grants as the API reads and writes them: assistant id → who it may ask.
pub type Grants = HashMap<String, Vec<String>>;
