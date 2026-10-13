//! The assistant's own browser.
//!
//! Until this, an assistant could *show* the user a page (`browser_open`, the panel) or
//! drive the whole desktop by pixels (`computer_use`). It could not work a website: sign
//! in once and stay signed in, fill a form by what its fields are called, hand the page to
//! the user at a sign-in wall and carry on when they hand it back.
//!
//! * [`computer`] — one browser per assistant: a Chrome on its own profile, snapshots with
//!   refs, actions, the live screen, kept frames, stop and reset.
//! * [`control`] — who has the wheel.
//! * [`snapshot`] — the accessibility tree as a short list the model can act on.
//! * [`proxy`] — the browser's only way out, so local-only and the Network page still hold.
//! * [`cdp`] / [`chrome`] — the protocol over a pipe, and starting the browser.
//! * [`keys`] — key names to the events Chrome acts on.
//!
//! Every action an assistant takes arrives through `core::gateway` like any other tool
//! call, so boundaries apply to clicks and keystrokes by what the server's own snapshot
//! says the element is.

pub mod cdp;
pub mod chrome;
pub mod computer;
pub mod control;
pub mod keys;
pub mod proxy;
pub mod snapshot;

#[cfg(test)]
mod live_tests;

pub use computer::{current_url, element_for};

/// The part of the tool guidance an assistant is given only when it has a browser: an
/// assistant is not told about hands it has not got.
pub const GUIDANCE: &str = "\n\n## Your browser\n\n\
You have your own web browser: a real Chrome that stays signed in to the sites the user has signed it into, and that \
the user can watch. When you need to act on a website — or read a page that needs a sign-in or JavaScript — open it \
with browser_navigate and answer from the text it returns. Don't tell the user to go and look: you have read it. To \
put a page in front of the user instead, use browser_open.\n\n\
To act on a page, call browser_snapshot first. It lists every field, button and link with a ref such as e4 and a \
snapshot_id; then call browser_type, browser_click or browser_key with those refs and that snapshot_id. Never invent \
a ref. If an action says the refs are out of date, the page changed: take a new snapshot. After submitting \
something, call browser_read and say what the page now says — not only that you clicked.\n\n\
Some pages need the user: a sign-in, a password, a code sent to their phone, a CAPTCHA. Then call \
browser_request_help and say exactly what they need to do. Calling it IS asking — writing 'please sign in and let me \
know' in your answer reaches nobody and leaves the page where it is. Never ask the user for a password or a code in \
this conversation. When you need ONE value only, call browser_request_secret with that field's ref: the user types it \
into a masked box that goes straight into the page, and you never see it.\n\n\
While the user has control of your browser, your actions are refused. That is not an error: wait, and continue when \
they hand it back. If a boundary refuses an action, say plainly what was stopped and stop — don't try another way to \
do the same thing.";
