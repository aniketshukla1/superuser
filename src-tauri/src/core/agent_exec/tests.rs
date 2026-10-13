use super::*;
    use serde_json::json;

    /// The assistant's browser, through the dispatcher — every path that must not start
    /// Chrome, or must stop before touching a page.
    mod browser;

    /// Numbers a picture draws — Phase 5. The file is the source of truth; the model
    /// naming it beats the model retyping it.
    mod drawing_from_a_file;

    /// The board, through the dispatcher — where the actor is the run's own
    /// session id rather than anything the model supplied.
    mod board;

    /// The shell approval flow: nothing reaches a shell without the mode, the
    /// session's grants, or the user saying so.
    mod shell_gate;

    /// The Coverage ledger: a check either ran, or the note says it didn't.
    mod coverage;

    /// The tool that used to lie. These four say what it does now.
    mod asking;

mod chart_drawn_without;
mod deck_edits_are;
mod ctx_for;
mod watch_can_be;
mod reading_synced_content;

use chart_drawn_without::*;
use ctx_for::*;
