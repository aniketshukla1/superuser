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
You have a web browser of your own: a real Chrome, kept signed in to the sites the user has signed it into, which \
the user can watch. Whenever a task means doing something on a website, or reading a page that needs a sign-in or \
JavaScript, load it with browser_navigate and answer from the text you get back. Don't send the user off to look: you \
have already read it. To show the user a page instead, use browser_open.\n\n\
Before acting on a page, call browser_snapshot. It lists the fields, buttons and links, each with a ref like e4, \
along with a snapshot_id; pass those refs and that snapshot_id to browser_type, browser_click or browser_key. Only use \
refs the snapshot gave you. When an action reports the refs are out of date, the page has changed, so snapshot again. \
Once you have submitted something, call browser_read and tell the user what the page says now, not just that you \
clicked.\n\n\
Some steps only the user can do: signing in, a password, a code texted to their phone, a CAPTCHA. For those, call \
browser_request_help with precise instructions. The call is the request: a sentence in your reply asking them to \
sign in reaches no one and changes nothing on the page. Never ask for a password or code in this conversation. When \
all you need is ONE value, call browser_request_secret on that field's ref; the user enters it in a hidden input that \
fills the page directly, and it never reaches you.\n\n\
While the user is in control of your browser, your actions will be refused. That isn't a failure: wait, and carry \
on once they give it back. If a boundary refuses an action, tell the user plainly what was blocked and stop there; \
don't look for another route to the same result.";
