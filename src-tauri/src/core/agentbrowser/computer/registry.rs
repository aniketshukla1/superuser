//! the registry

mod get;
mod computer;
mod note_of;
mod event_loop;
mod press_key;

pub use get::*;
pub use note_of::*;
pub(super) use event_loop::*;
pub(super) use press_key::*;
