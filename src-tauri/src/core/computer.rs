//! Computer-use primitives: the app's "hands and eyes" for driving the desktop.
//!
//! - `screenshot()` grabs the screen and downscales it to a model-friendly size,
//!   returning base64 PNG + the scale needed to map model coordinates back to the
//!   real display.
//! - `execute()` performs a single low-level action (move / click / type / key /
//!   scroll) via synthesized input events.
//! - `accessibility_trusted()` reports whether macOS has granted us control.
//!
//! Everything runs locally; the agent loop (see commands::computer_use_run) is
//! what decides *what* to do — this module only senses and acts.

use enigo::{
    Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings,
};
use serde::Deserialize;

mod screenshot;
mod execute;

pub use screenshot::*;
pub use execute::*;
