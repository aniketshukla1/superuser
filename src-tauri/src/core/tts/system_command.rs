use super::*;

/// Where speech comes from. The tiers are deliberately ordered by what they cost
/// the user: the system voice is already installed and free, the local neural
/// engines are a download, and the cloud one is a key plus text leaving the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// The voice the operating system already ships. Resolved per platform, so this
    /// is not a macOS-shaped hole on Windows.
    System,
    /// Piper — local neural, a binary plus per-voice models.
    Piper,
    /// Kokoro — local ONNX.
    Kokoro,
    /// A hosted voice. Text leaves the machine, so it answers to the local-only
    /// switch like every other egress.
    Cloud,
}

impl Engine {
    pub fn parse(s: &str) -> Engine {
        match s.trim().to_ascii_lowercase().as_str() {
            "piper" => Engine::Piper,
            "kokoro" => Engine::Kokoro,
            "cloud" | "elevenlabs" | "openai" => Engine::Cloud,
            // Anything unrecognised falls to the engine that needs no setup, which
            // is the one that cannot leave the user with silence.
            _ => Engine::System,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Engine::System => "system",
            Engine::Piper => "piper",
            Engine::Kokoro => "kokoro",
            Engine::Cloud => "cloud",
        }
    }

    /// Whether using this engine sends the text off the machine.
    pub fn leaves_machine(&self) -> bool {
        matches!(self, Engine::Cloud)
    }
}

/// The command that speaks `text` in the OS's own voice.
///
/// Each platform's built-in speech has a different entry point; naming them all
/// here keeps the rest of the module from growing `#[cfg]` branches.
pub fn system_command(voice: &str, text: &str) -> (String, Vec<String>) {
    let voice = voice.trim();
    if cfg!(target_os = "macos") {
        let mut args: Vec<String> = Vec::new();
        if !voice.is_empty() {
            args.push("-v".into());
            args.push(voice.to_string());
        }
        args.push(text.to_string());
        return ("say".into(), args);
    }
    if cfg!(target_os = "windows") {
        // SAPI through PowerShell. The text is passed as a single-quoted literal
        // with '' escaping, so an apostrophe cannot end the string and turn the
        // rest of a spoken sentence into commands.
        let safe = text.replace('\'', "''");
        let select = if voice.is_empty() {
            String::new()
        } else {
            format!("$s.SelectVoice('{}');", voice.replace('\'', "''"))
        };
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; {select} $s.Speak('{safe}')"
        );
        return ("powershell".into(), vec!["-NoProfile".into(), "-Command".into(), script]);
    }
    // Linux and the rest: speech-dispatcher is the common front end, and it takes
    // its text on argv like the others.
    let mut args: Vec<String> = Vec::new();
    if !voice.is_empty() {
        args.push("-o".into());
        args.push(voice.to_string());
    }
    args.push(text.to_string());
    ("spd-say".into(), args)
}

/// How to run one neural engine.
///
/// Kept as data rather than code because these CLIs move faster than we ship: a
/// flag rename in piper or a different kokoro front end would otherwise mean a
/// rebuild. `engines.json` in the speech folder overrides any of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSpec {
    pub binary: String,
    /// Placeholders: `{model}`, `{voice}`, `{out}`, `{text}`.
    pub args: Vec<String>,
    /// Whether the text goes in on stdin rather than as an argument.
    pub text_on_stdin: bool,
}

/// What we run when the user has not said otherwise.
pub fn default_spec(engine: Engine) -> Option<EngineSpec> {
    match engine {
        Engine::Piper => Some(EngineSpec {
            binary: "piper".into(),
            args: vec!["--model".into(), "{model}".into(), "--output_file".into(), "{out}".into()],
            text_on_stdin: true,
        }),
        Engine::Kokoro => Some(EngineSpec {
            binary: "kokoro-tts".into(),
            args: vec!["-".into(), "{out}".into(), "--voice".into(), "{voice}".into()],
            text_on_stdin: true,
        }),
        _ => None,
    }
}

pub fn speech_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("speech")
}

pub(super) fn spec_overrides(data_dir: &Path) -> std::collections::HashMap<String, EngineSpec> {
    std::fs::read_to_string(speech_dir(data_dir).join("engines.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The spec actually used: the user's override if present, else the default.
pub fn spec_for(data_dir: &Path, engine: Engine) -> Option<EngineSpec> {
    spec_overrides(data_dir).remove(engine.as_str()).or_else(|| default_spec(engine))
}

/// Where an engine's model files live.
pub fn model_dir(data_dir: &Path, engine: Engine) -> PathBuf {
    speech_dir(data_dir).join(engine.as_str())
}

/// The voices installed for an engine.
///
/// Piper ships one `.onnx` per voice, so the filenames ARE the voice list. Kokoro
/// carries many voices inside one model, so its names cannot be read off the disk
/// and come from the override file (or the user typing one).
pub fn installed_voices(data_dir: &Path, engine: Engine) -> Vec<String> {
    let dir = model_dir(data_dir, engine);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("onnx") {
                return None;
            }
            p.file_stem().and_then(|x| x.to_str()).map(|s| s.to_string())
        })
        .collect();
    out.sort();
    out
}

/// The model file for a voice, if we can name one.
pub fn model_for(data_dir: &Path, engine: Engine, voice: &str) -> Option<PathBuf> {
    let dir = model_dir(data_dir, engine);
    if !voice.trim().is_empty() {
        let named = dir.join(format!("{}.onnx", voice.trim()));
        if named.exists() {
            return Some(named);
        }
    }
    // Fall back to whatever single model is installed — with one voice present,
    // asking the user to name it is ceremony.
    installed_voices(data_dir, engine).first().map(|v| dir.join(format!("{v}.onnx")))
}

/// Fill the placeholders in a spec's arguments.
pub fn render_args(spec: &EngineSpec, model: &str, voice: &str, out: &str, text: &str) -> Vec<String> {
    spec.args
        .iter()
        .map(|a| {
            a.replace("{model}", model)
                .replace("{voice}", voice)
                .replace("{out}", out)
                .replace("{text}", text)
        })
        .collect()
}
