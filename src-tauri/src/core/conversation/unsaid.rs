use super::*;

pub(super) fn calibration_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("speech").join("calibration.json")
}

pub fn save_calibration(data_dir: &std::path::Path, c: &Calibration) -> std::io::Result<()> {
    let path = calibration_path(data_dir);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    // Atomically, like every other store here: a truncating write leaves a window
    // where a reader sees an empty file, and an empty calibration reads as a
    // perfectly silent room — the tightest possible thresholds.
    crate::core::store::write_json_atomic(&path, c)
}

/// The saved measurement, if there is a usable one.
///
/// A file that no longer parses, or one whose thresholds have been hand-edited into
/// nonsense, reads as absent — falling back to measuring is always better than
/// running with a detector that cannot start or cannot stop.
pub fn load_calibration(data_dir: &std::path::Path) -> Option<Calibration> {
    let c: Calibration = std::fs::read_to_string(calibration_path(data_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())?;
    let sane = c.start_level > c.end_level
        && c.end_level > 0.0
        && c.start_level <= 1.0
        && c.noise_floor >= 0.0;
    sane.then_some(c)
}

pub fn forget_calibration(data_dir: &std::path::Path) {
    let _ = std::fs::remove_file(calibration_path(data_dir));
}

/// The floor of a set of level readings.
///
/// The MEDIAN, not the mean or the minimum: a mean is dragged up by any speech that
/// lands in the window, and a minimum is dragged down by the gap between two words.
/// The middle of a mostly-quiet sample is the room.
pub fn floor_of(mut samples: Vec<f32>) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

/// How long the assistant's voice keeps counting as "still playing" after its
/// audio stops.
///
/// Two things happen in that window. Between the sentences of one answer the
/// player process has already exited while the next has not started, so a check
/// for "is it speaking" says no in the middle of a reply. And a room keeps ringing
/// after a speaker goes quiet. Treating both as still-speaking is what stops a
/// gap between sentences from being heard as the user's turn.
pub const ECHO_HANGOVER_MS: u32 = 400;

/// Tracks whether the assistant's voice is still in the room.
///
/// Without acoustic echo cancellation the microphone hears the speakers, so this
/// decides both when to deafen the turn detector and which recorded audio can
/// never belong to the user.
#[derive(Debug, Clone)]
pub struct EchoGate {
    pub(super) hangover_ms: u32,
    pub(super) since_ms: u32,
    pub(super) was_speaking: bool,
}

impl Default for EchoGate {
    fn default() -> Self {
        EchoGate { hangover_ms: ECHO_HANGOVER_MS, since_ms: u32::MAX, was_speaking: false }
    }
}

impl EchoGate {
    pub fn with_hangover(ms: u32) -> Self {
        EchoGate { hangover_ms: ms, ..Default::default() }
    }

    /// Advance by one frame. Returns whether the assistant's voice should still be
    /// treated as present.
    pub fn tick(&mut self, assistant_speaking: bool, frame_ms: u32) -> bool {
        if assistant_speaking {
            self.since_ms = 0;
            self.was_speaking = true;
            return true;
        }
        if self.was_speaking {
            self.since_ms = self.since_ms.saturating_add(frame_ms);
            if self.since_ms < self.hangover_ms {
                return true;
            }
            self.was_speaking = false;
        }
        false
    }

    pub fn active(&self) -> bool {
        self.was_speaking && self.since_ms < self.hangover_ms
    }
}

/// Where a user turn may begin in the rolling buffer.
///
/// Three floors, and the highest wins: don't re-read audio a finished turn already
/// consumed, don't reach back past where the assistant's own voice was recorded,
/// and otherwise rewind by the pre-roll so the first syllable is not clipped.
///
/// The barrier is what makes speakers safe. Without it, the microphone's recording
/// of the assistant is just audio in the buffer, and a turn that starts near it
/// hands the assistant's own words to the transcriber — which then arrive as if
/// the user had said them.
pub fn turn_start_index(buffered: usize, pre_roll: usize, consumed: usize, barrier: usize) -> usize {
    buffered.saturating_sub(pre_roll).max(consumed).max(barrier).min(buffered)
}

/// What the assistant was saying when it got cut off.
///
/// Being interrupted is normal in conversation; *pretending it did not happen* is
/// what makes a machine feel like a machine. Keeping the unsaid remainder means
/// "sorry, go on" can actually go on, and the assistant never claims to have said
/// something the user never heard.
#[derive(Debug, Default, Clone)]
pub struct Unsaid {
    pub(super) spoken: String,
    pub(super) remaining: String,
}

impl Unsaid {
    /// Begin an answer: all of it is still to say.
    pub fn begin(full: &str) -> Unsaid {
        Unsaid { spoken: String::new(), remaining: full.trim().to_string() }
    }

    /// Record that a chunk actually reached the speaker.
    pub fn spoke(&mut self, chunk: &str) {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            return;
        }
        if !self.spoken.is_empty() {
            self.spoken.push(' ');
        }
        self.spoken.push_str(chunk);
        let rest = self.remaining.trim_start();
        self.remaining = match rest.strip_prefix(chunk) {
            Some(tail) => tail.trim_start().to_string(),
            // The chunk was reworded on its way to the voice (markdown stripped,
            // say); fall back to dropping what we can rather than losing the thread.
            None => rest.to_string(),
        };
    }

    pub fn was_interrupted(&self) -> bool {
        !self.remaining.trim().is_empty()
    }

    /// What the user actually heard.
    pub fn heard(&self) -> &str {
        &self.spoken
    }

    /// What they did not.
    pub fn unheard(&self) -> &str {
        self.remaining.trim()
    }

    /// A line for the transcript, so the written record matches what was said out
    /// loud rather than what was generated.
    pub fn note(&self) -> Option<String> {
        if !self.was_interrupted() {
            return None;
        }
        Some(format!("(interrupted — unsaid: {})", self.unheard()))
    }
}
