//! Speech in — local transcription, with nothing leaving the machine.
//!
//! The GUI has had a complete voice-input surface for a while (download the model,
//! verify it, run a test recording, then a mic button in the composer) and nothing
//! behind it: the commands it invoked did not exist, so the button could only ever
//! report that dictation was unavailable. This is the other half.
//!
//! Whisper wants 16 kHz mono f32. Microphones do not: they hand back 44.1 or 48 kHz,
//! often in stereo, sometimes as i16. Everything here is about meeting it in the
//! middle without sending a byte anywhere.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod to_whisper_mono;
mod model_looks_complete;

pub use to_whisper_mono::*;
pub use model_looks_complete::*;
