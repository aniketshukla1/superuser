//! Gates — the pushback a tool call gets when the model made something up.
//!
//! A tool layer normally has one job: do the thing, or report why it couldn't. That
//! is enough when the caller is a program. It is not enough when the caller is a
//! model, because a model's failure mode is not calling the wrong tool — it is
//! calling the right tool with **invented arguments**, confidently, and reading the
//! result as confirmation. A chart of numbers nobody read still renders. A fetch of
//! `example.com` still returns a page. Nothing errors, and the run produces
//! something that looks like evidence.
//!
//! So these gates sit in front of the executor and refuse, with a message written
//! *to the model* saying what was wrong and which tool to reach for instead. Two
//! rules they all follow:
//!
//! 1. **Block only what cannot be true.** A gate that fires on a real path is worse
//!    than no gate: it teaches the model to route around the tool layer. Every rule
//!    below is checkable — a reserved domain, a path the user never mentioned, a
//!    file this run never opened — not a guess about intent.
//! 2. **Always name the way forward.** A refusal that ends in "don't" leaves the
//!    model to invent a second time. Each one names `ask_user`, `list_dir` or
//!    `sheet_read` by name, because the next call is the thing being steered.
//!
//! The gate that matters most is [`grounding`]. The others catch sloppiness; that
//! one catches fabrication with a chart wrapped around it.

use serde_json::Value;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod stall_gate {
    use super::*;

    #[test]
    fn a_call_that_keeps_saying_the_same_thing_is_stopped() {
        assert!(stalled("grep", 0).is_none(), "a first result is information");
        assert!(stalled("grep", 1).is_none(), "one repeat could still be a real re-read");
        let refusal = stalled("grep", 2).expect("two identical results is a stall");
        assert!(refusal.starts_with("error:"), "the loop reads this as a failed call");
        assert!(refusal.contains("grep"), "name the tool that is stuck");
        // The message has to offer a way out. A model that had a next move would
        // have made it, so "you are looping" alone leaves it exactly where it was.
        assert!(
            refusal.contains("answer now") || refusal.contains("something genuinely different"),
            "the refusal must suggest a move, not just diagnose: {refusal}"
        );
        assert!(refusal.contains("say so plainly"), "stopping honestly must be an allowed answer");
    }

    /// The whole design rests on this: keying on the RESULT rather than on the call
    /// is what lets re-reading a file you just edited stay legal.
    #[test]
    fn the_gate_is_about_the_result_not_the_call() {
        // Two identical calls whose results differed never reach a count of 2 —
        // `note_result` resets to 1 on a changed digest — so the gate is silent.
        assert!(stalled("read_file", 1).is_none());
    }
}

mod grounding;
mod stalled;
mod prompt_block;

pub use grounding::*;
pub use stalled::*;
pub use prompt_block::*;
