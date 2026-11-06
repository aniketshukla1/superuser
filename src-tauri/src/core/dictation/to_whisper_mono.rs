use super::*;

/// What whisper expects, in Hz.
pub const TARGET_RATE: u32 = 16_000;

/// The model we install. `base.en` is the smallest one that transcribes ordinary
/// speech well enough to trust; `tiny` mishears often enough that the user ends up
/// correcting more than they dictated, which defeats the point.
pub const MODEL_NAME: &str = "ggml-base.en.bin";

pub const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";

/// Approximate download size, shown before the user commits to it.
pub const MODEL_BYTES: u64 = 147_951_465;

/// A truncated download is the common failure, and a half-file loads as garbage
/// rather than failing loudly, so anything well under the real size is rejected.
pub(super) const MIN_PLAUSIBLE_BYTES: u64 = 100_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictationStatus {
    /// Whether this machine can dictate at all, and if not, why.
    pub supported: bool,
    pub compatibility_reason: Option<String>,
    pub device_summary: String,
    /// The speech model: which one, its size, and how far along it is.
    pub model_name: String,
    pub model_bytes: u64,
    pub model_installed: bool,
    pub model_verified: bool,
    pub download_in_progress: bool,
    /// Right now.
    pub test_passed: bool,
    pub recording: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
}

pub fn model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("speech").join(MODEL_NAME)
}

pub(super) fn verified_marker(data_dir: &Path) -> PathBuf {
    data_dir.join("speech").join(".verified")
}

pub(super) fn tested_marker(data_dir: &Path) -> PathBuf {
    data_dir.join("speech").join(".tested")
}

/// Mix down to mono and resample to 16 kHz.
///
/// The averaging matters. Picking every Nth sample is one line shorter and aliases
/// badly — high frequencies fold down into the speech band as a metallic hiss, and
/// whisper transcribes the hiss. Averaging the samples that fall inside each output
/// window is a crude low-pass, but it is the difference between a transcript and a
/// guess.
pub fn to_whisper_mono(samples: &[f32], channels: u16, rate: u32) -> Vec<f32> {
    if samples.is_empty() || channels == 0 || rate == 0 {
        return Vec::new();
    }
    let ch = channels as usize;
    let frames = samples.len() / ch;
    if frames == 0 {
        return Vec::new();
    }
    let mono: Vec<f32> = (0..frames)
        .map(|f| {
            let sum: f32 = (0..ch).map(|c| samples[f * ch + c]).sum();
            sum / ch as f32
        })
        .collect();
    if rate == TARGET_RATE {
        return mono;
    }
    let ratio = rate as f64 / TARGET_RATE as f64;
    let out_len = ((mono.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * ratio).floor() as usize;
        let end = (((i + 1) as f64 * ratio).ceil() as usize).min(mono.len()).max(start + 1);
        let window = &mono[start..end.min(mono.len())];
        if window.is_empty() {
            break;
        }
        out.push(window.iter().sum::<f32>() / window.len() as f32);
    }
    out
}

/// Loudness 0..1 for the composer's waveform. RMS rather than peak: peak jumps on
/// a single click and reads as a spike the user did not make.
pub fn level_of(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sq / samples.len() as f32).sqrt();
    // Speech sits low in a linear scale; this brings a normal voice to roughly
    // mid-meter without clipping the top for a shout.
    (rms * 4.0).clamp(0.0, 1.0)
}

/// Whisper emits its own bracketed stage directions for silence and noise. They are
/// not words the user said, and pasting "[BLANK_AUDIO]" into the composer is worse
/// than pasting nothing.
pub fn clean_transcript(raw: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in raw.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Everything the running app holds for dictation.
#[derive(Default)]
pub struct Dictation {
    pub recording: Arc<AtomicBool>,
    /// Hands-free mode: the mic stays open and turns are detected rather than
    /// bracketed by a button.
    pub conversing: Arc<AtomicBool>,
    pub downloading: Arc<AtomicBool>,
    pub cancel_download: Arc<AtomicBool>,
    /// Latest loudness, as milli-units so it fits an atomic.
    pub level_milli: Arc<AtomicU32>,
    /// Captured audio, already mono at the device's rate.
    pub samples: Arc<Mutex<Vec<f32>>>,
    pub source_rate: Arc<AtomicU32>,
    /// Held only to keep the input stream alive; dropping it stops capture.
    pub stream: Mutex<Option<Box<dyn StreamHandle>>>,
}

/// A live capture, kept behind a trait so the platform stream type does not leak
/// into `AppState` (and so tests can stand in for one).
pub trait StreamHandle: Send {
    fn stop(self: Box<Self>);
}

impl Dictation {
    pub fn level(&self) -> f32 {
        self.level_milli.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn set_level(&self, v: f32) {
        self.level_milli.store((v.clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
    }

    pub fn status(&self, data_dir: &Path, device_summary: String, unsupported: Option<String>) -> DictationStatus {
        let path = model_path(data_dir);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let installed = size > 0;
        DictationStatus {
            recording: self.recording.load(Ordering::Relaxed),
            model_installed: installed,
            // A file that is present but too small was a truncated download; it is
            // reported as unverified so the UI offers repair rather than letting a
            // corrupt model fail at the first recording.
            model_verified: installed && size >= MIN_PLAUSIBLE_BYTES && verified_marker(data_dir).exists(),
            test_passed: tested_marker(data_dir).exists(),
            download_in_progress: self.downloading.load(Ordering::Relaxed),
            model_name: MODEL_NAME.to_string(),
            model_bytes: MODEL_BYTES,
            supported: unsupported.is_none(),
            device_summary,
            compatibility_reason: unsupported,
        }
    }
}

/// Record that the model on disk is the size it should be.
pub fn mark_verified(data_dir: &Path) -> std::io::Result<()> {
    let dir = data_dir.join("speech");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(verified_marker(data_dir), b"1")
}

pub fn mark_tested(data_dir: &Path) -> std::io::Result<()> {
    let dir = data_dir.join("speech");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(tested_marker(data_dir), b"1")
}

/// Drop the model and both markers, so the next install starts clean.
pub fn forget_model(data_dir: &Path) {
    let _ = std::fs::remove_file(model_path(data_dir));
    let _ = std::fs::remove_file(verified_marker(data_dir));
    let _ = std::fs::remove_file(tested_marker(data_dir));
}
