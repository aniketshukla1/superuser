use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TurnConfig {
    /// Level at which speech is considered to have begun.
    pub start_level: f32,
    /// Level below which the room counts as quiet. Deliberately lower than
    /// `start_level` — the gap is the hysteresis that stops the detector flapping.
    pub end_level: f32,
    /// How long sound must persist before it counts as somebody talking.
    pub min_speech_ms: u32,
    /// How long quiet must last before the turn is over. This is the number that
    /// decides whether the assistant feels attentive or impatient.
    pub tail_ms: u32,
}

impl TurnConfig {
    /// Thresholds fitted to the room this microphone is actually in.
    ///
    /// Fixed thresholds cannot work. A level meter reads a mixture of the speaker's
    /// voice, the room, and whatever gain the device applies, and that mixture
    /// differs by an order of magnitude between a laptop mic in an office and a
    /// headset in a quiet room. Ship one number and half your users get a detector
    /// that never starts a turn, and the other half one that never ENDS one —
    /// because ending requires the level to fall below a floor the room never goes
    /// under. The second failure is the crueller of the two: speech is heard, the
    /// turn opens, and nothing is ever transcribed, because a turn that does not
    /// end is a turn that never gets sent.
    ///
    /// So both thresholds are placed relative to the measured floor, with minimums
    /// so a silent room cannot collapse them onto the noise of the microphone
    /// itself.
    pub fn for_noise_floor(floor: f32) -> TurnConfig {
        let floor = floor.clamp(0.0, 0.6);
        let d = TurnConfig::default();
        // Ending needs to be reachable: comfortably above the floor, or a quiet
        // room's own hiss would hold every turn open.
        let end_level = (floor * 1.6).max(0.02).min(0.45);
        // Starting needs to be out of reach of the room, or it triggers on nothing.
        // Capped as well as floored: the meter is clamped to 1.0, so a threshold
        // above that is not "strict", it is deaf — a calibration taken while
        // somebody was shouting would otherwise leave a detector nothing can reach.
        let start_level = (floor * 2.8).max(end_level * 1.8).max(0.05).min(0.85);
        TurnConfig { start_level, end_level, ..d }
    }

    /// The same detector, deafened a little, for use *while the assistant is
    /// speaking*.
    ///
    /// Without echo cancellation the microphone hears the speakers, so the voice's
    /// own first sentence trips barge-in and it shuts itself up — a machine that
    /// interrupts itself. Two changes fix it without AEC: interrupting must be
    /// louder than merely talking, and it must last long enough that the assistant's
    /// own cadence cannot fake it. The cost is that a whisper will not interrupt;
    /// the alternative is an assistant that cannot finish a sentence.
    pub fn while_speaking(self) -> TurnConfig {
        TurnConfig {
            start_level: (self.start_level * 2.2).min(0.85),
            end_level: self.end_level * 2.0,
            min_speech_ms: self.min_speech_ms.max(260),
            tail_ms: self.tail_ms,
        }
    }
}

impl Default for TurnConfig {
    fn default() -> Self {
        TurnConfig {
            start_level: 0.10,
            end_level: 0.05,
            min_speech_ms: 120,
            // ~0.7s: long enough to pause for thought mid-sentence, short enough
            // that the reply doesn't feel like it arrived by post.
            tail_ms: 700,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEvent {
    /// Somebody started talking. While the assistant is speaking, this is barge-in.
    SpeechStarted,
    /// The turn is over — everything up to here is what they said.
    SpeechEnded,
    Nothing,
}

/// Watches the level meter and reports turn boundaries.
#[derive(Debug, Clone)]
pub struct TurnDetector {
    pub(super) cfg: TurnConfig,
    pub(super) in_speech: bool,
    pub(super) loud_ms: u32,
    pub(super) quiet_ms: u32,
}

impl TurnDetector {
    pub fn new(cfg: TurnConfig) -> Self {
        TurnDetector { cfg, in_speech: false, loud_ms: 0, quiet_ms: 0 }
    }

    pub fn speaking(&self) -> bool {
        self.in_speech
    }

    /// Feed one frame of audio level. `frame_ms` is how much time it represents.
    pub fn push(&mut self, level: f32, frame_ms: u32) -> TurnEvent {
        if self.in_speech {
            if level < self.cfg.end_level {
                self.quiet_ms += frame_ms;
                if self.quiet_ms >= self.cfg.tail_ms {
                    self.in_speech = false;
                    self.quiet_ms = 0;
                    self.loud_ms = 0;
                    return TurnEvent::SpeechEnded;
                }
            } else {
                // Any sound above the floor resets the countdown: a pause only
                // ends a turn if it is uninterrupted.
                self.quiet_ms = 0;
            }
            return TurnEvent::Nothing;
        }
        if level >= self.cfg.start_level {
            self.loud_ms += frame_ms;
            if self.loud_ms >= self.cfg.min_speech_ms {
                self.in_speech = true;
                self.quiet_ms = 0;
                self.loud_ms = 0;
                return TurnEvent::SpeechStarted;
            }
        } else {
            // A single loud frame surrounded by quiet was a noise, not a word.
            self.loud_ms = 0;
        }
        TurnEvent::Nothing
    }

    /// Forget any partial turn — used when the mic stops or the mode changes.
    pub fn reset(&mut self) {
        self.in_speech = false;
        self.loud_ms = 0;
        self.quiet_ms = 0;
    }
}

/// How far back to reach when a turn starts, in milliseconds.
///
/// The detector cannot fire until speech has persisted for `min_speech_ms`, so by
/// the time a turn "starts" its first syllable is already in the past. Cutting the
/// recording at the detection point clips the leading consonant off every sentence
/// and whisper hears "ook the room" for "book the room". Rewinding a little is what
/// makes the transcript begin at the word the user actually began with.
pub const PRE_ROLL_MS: u32 = 300;

/// Samples to rewind by, for a given rate.
pub fn pre_roll_samples(rate: u32) -> usize {
    (rate as usize * PRE_ROLL_MS as usize) / 1000
}

/// A measurement of one room, kept so it is made once rather than every time the
/// user starts talking.
///
/// Measuring at the start of each conversation cost three quarters of a second of
/// dead air before every session, and quietly assumed the user happened to be
/// silent at that moment. Doing it once, deliberately, in the place where voice is
/// configured is both faster and more honest: the user is told what is being
/// measured, sees the result, and can redo it when the room changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub noise_floor: f32,
    pub start_level: f32,
    pub end_level: f32,
    /// Unix seconds, so the UI can say how old the measurement is — a calibration
    /// from a different room is worse than none.
    pub measured_at: f64,
}

impl Calibration {
    pub fn from_floor(floor: f32, now: f64) -> Calibration {
        let cfg = TurnConfig::for_noise_floor(floor);
        Calibration {
            noise_floor: floor,
            start_level: cfg.start_level,
            end_level: cfg.end_level,
            measured_at: now,
        }
    }

    pub fn to_config(&self) -> TurnConfig {
        TurnConfig {
            start_level: self.start_level,
            end_level: self.end_level,
            ..TurnConfig::default()
        }
    }
}
