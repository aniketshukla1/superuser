use super::*;

/// Which hosted voice to use.
///
/// Kept beside the neural engines' spec for the same reason: providers rename
/// models faster than we ship, and a voice id is a per-user choice, not a constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudVoice {
    /// "elevenlabs" or "openai".
    pub provider: String,
    /// The provider's voice id.
    pub voice: String,
    pub model: String,
}

impl Default for CloudVoice {
    fn default() -> Self {
        CloudVoice {
            provider: "openai".into(),
            voice: "alloy".into(),
            model: "gpt-4o-mini-tts".into(),
        }
    }
}

pub fn cloud_config(data_dir: &Path) -> CloudVoice {
    std::fs::read_to_string(speech_dir(data_dir).join("cloud.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The key for a hosted voice provider.
///
/// OpenAI reuses the key the user already gave for models rather than asking for a
/// second one for the same account; ElevenLabs has no model provider here, so it
/// carries its own.
pub fn cloud_key(provider: &str) -> Option<String> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openai" => crate::core::secrets::get_provider_key("openai")
            .or_else(|| crate::core::secrets::get_integration_token("openai")),
        other => crate::core::secrets::get_integration_token(other),
    }
}

/// One request to a hosted voice: url, headers and body.
///
/// Built as data so it can be checked without a network or a key — the shape of
/// these requests is exactly where a typo costs an afternoon.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: serde_json::Value,
    /// What the response will contain, so the temp file gets an extension the
    /// system player recognises.
    pub extension: &'static str,
}

pub fn cloud_request(cfg: &CloudVoice, key: &str, text: &str) -> Result<CloudRequest, String> {
    match cfg.provider.trim().to_ascii_lowercase().as_str() {
        "elevenlabs" => {
            if cfg.voice.trim().is_empty() {
                return Err("ElevenLabs needs a voice id — set one in speech/cloud.json".into());
            }
            Ok(CloudRequest {
                url: format!(
                    "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=mp3_44100_128",
                    cfg.voice.trim()
                ),
                headers: vec![
                    ("xi-api-key".into(), key.to_string()),
                    ("Content-Type".into(), "application/json".into()),
                ],
                body: json!({ "text": text, "model_id": cfg.model }),
                extension: "mp3",
            })
        }
        "openai" => Ok(CloudRequest {
            url: "https://api.openai.com/v1/audio/speech".into(),
            headers: vec![
                ("Authorization".into(), format!("Bearer {key}")),
                ("Content-Type".into(), "application/json".into()),
            ],
            // wav rather than mp3: it is what the local engines produce, so one
            // player path covers every engine.
            body: json!({ "model": cfg.model, "input": text, "voice": cfg.voice, "response_format": "wav" }),
            extension: "wav",
        }),
        other => Err(format!("`{other}` isn't a voice provider I know — use openai or elevenlabs")),
    }
}

/// Play a finished wav. A separate process on purpose: killing it is barge-in.
pub fn play_command(path: &str) -> (String, Vec<String>) {
    if cfg!(target_os = "macos") {
        return ("afplay".into(), vec![path.to_string()]);
    }
    if cfg!(target_os = "windows") {
        let safe = path.replace('\'', "''");
        return (
            "powershell".into(),
            vec![
                "-NoProfile".into(),
                "-Command".into(),
                format!("(New-Object Media.SoundPlayer '{safe}').PlaySync()"),
            ],
        );
    }
    ("aplay".into(), vec![path.to_string()])
}

/// Whether an engine can actually run here, and why not when it cannot.
#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub engine: String,
    pub available: bool,
    pub binary: String,
    pub voices: Vec<String>,
    pub reason: Option<String>,
}

/// Is `binary` runnable? Resolved through a login shell, because pip and brew put
/// things on a PATH an app launched from Finder never sees.
pub fn binary_exists(binary: &str) -> bool {
    if binary.trim().is_empty() {
        return false;
    }
    #[cfg(windows)]
    let probe = Command::new("cmd").arg("/C").arg(format!("where {binary}")).output();
    #[cfg(not(windows))]
    let probe = Command::new("sh").arg("-lc").arg(format!("command -v {binary}")).output();
    probe.map(|o| o.status.success()).unwrap_or(false)
}

pub fn engine_status(data_dir: &Path, engine: Engine) -> EngineStatus {
    if engine == Engine::System {
        let (binary, _) = system_command("", "x");
        return EngineStatus {
            engine: engine.as_str().into(),
            available: true,
            binary,
            voices: Vec::new(),
            reason: None,
        };
    }
    if engine == Engine::Cloud {
        // Judged by whether it has a key, not a binary — there is nothing to install.
        let cfg = cloud_config(data_dir);
        let has_key = cloud_key(&cfg.provider).is_some();
        return EngineStatus {
            engine: engine.as_str().into(),
            available: has_key,
            binary: cfg.provider.clone(),
            voices: if cfg.voice.is_empty() { Vec::new() } else { vec![cfg.voice.clone()] },
            reason: (!has_key).then(|| format!("no API key for {} — add one in Settings", cfg.provider)),
        };
    }
    let Some(spec) = spec_for(data_dir, engine) else {
        return EngineStatus {
            engine: engine.as_str().into(),
            available: false,
            binary: String::new(),
            voices: Vec::new(),
            reason: Some("no command is configured for this engine".into()),
        };
    };
    let voices = installed_voices(data_dir, engine);
    let has_binary = binary_exists(&spec.binary);
    let needs_model = engine == Engine::Piper;
    let reason = if !has_binary {
        Some(format!("`{}` isn't installed, or isn't on this machine's PATH", spec.binary))
    } else if needs_model && voices.is_empty() {
        Some(format!("no voice models in {}", model_dir(data_dir, engine).display()))
    } else {
        None
    };
    EngineStatus {
        engine: engine.as_str().into(),
        available: reason.is_none(),
        binary: spec.binary,
        voices,
        reason,
    }
}
