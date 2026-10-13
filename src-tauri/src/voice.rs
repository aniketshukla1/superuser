//! The voice commands the GUI has been calling into thin air.
//!
//! `Composer.tsx` and `SettingsView.tsx` were written against eleven Tauri commands
//! — download, verify, test, record, transcribe, level — and none of them existed,
//! so the mic button's only possible outcome was "Voice dictation is unavailable."
//! This is that contract, honoured.
//!
//! Nothing here reaches the network except the one-time model download, and that
//! goes through the egress chokepoint like everything else.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{Emitter, Manager};

use crate::core::dictation::{
    self, level_of, to_whisper_mono, DictationStatus, DownloadProgress, StreamHandle,
};
use crate::AppState;

#[cfg(test)]
mod tests;

mod list_input_devices;
mod start_dictation;
mod download_dictation_model;
mod speech_out;
mod hands_free_conversation;
mod what_user_is;
mod desktop_behaviour_gui;

pub use list_input_devices::*;
pub use start_dictation::*;
pub use download_dictation_model::*;
pub use speech_out::*;
pub use hands_free_conversation::*;
pub use what_user_is::*;
pub use desktop_behaviour_gui::*;
