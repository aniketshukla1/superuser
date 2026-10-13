use super::*;

impl Speaker {
    /// Speak one chunk, replacing whatever is being said now.
    ///
    /// The system voice speaks directly. A neural engine synthesises to a wav and
    /// then plays it, which costs a beat of latency but keeps the property that
    /// matters: the thing making sound is a process we can kill, so barge-in still
    /// lands mid-word.
    ///
    /// A missing engine is reported, never quietly swapped for the system voice.
    /// Silently substituting would mean the voice the user chose was not the voice
    /// they heard, and nothing on screen would say so.
    pub fn speak(&self, data_dir: &Path, engine: Engine, voice: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        self.stop();
        let (program, args) = match engine {
            Engine::System => system_command(voice, text),
            Engine::Piper | Engine::Kokoro => {
                let wav = self.synthesize(data_dir, engine, voice, text)?;
                // Between synthesis and playback the user may already have started
                // talking; playing now would be the assistant speaking over them.
                if self.cancelled() {
                    let _ = std::fs::remove_file(&wav);
                    return Ok(());
                }
                let (p, a) = play_command(&wav.to_string_lossy());
                self.temp.lock().unwrap().replace(wav);
                (p, a)
            }
            Engine::Cloud => {
                let wav = self.fetch_cloud(data_dir, text)?;
                if self.cancelled() {
                    let _ = std::fs::remove_file(&wav);
                    return Ok(());
                }
                let (p, a) = play_command(&wav.to_string_lossy());
                self.temp.lock().unwrap().replace(wav);
                (p, a)
            }
        };
        let child = Command::new(&program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("couldn't start `{program}`: {e}"))?;
        *self.current.lock().unwrap() = Some(child);
        Ok(())
    }

    /// Ask a hosted voice for one sentence.
    ///
    /// Through the egress chokepoint like every other outbound call, so it is
    /// recorded and answers to the same policy. The text leaves the machine — that
    /// is the whole trade of this tier, and it is why the caller checks local-only
    /// before ever getting here.
    pub fn fetch_cloud(&self, data_dir: &Path, text: &str) -> Result<PathBuf, String> {
        let cfg = cloud_config(data_dir);
        let key = cloud_key(&cfg.provider).ok_or_else(|| {
            format!(
                "No API key for {} — add one in Settings, or pick a local voice",
                cfg.provider
            )
        })?;
        let req = cloud_request(&cfg, &key, text)?;
        let client = crate::core::egress::client("tts");
        let bytes = tauri::async_runtime::block_on(async move {
            let mut r = client.post(&req.url).body(req.body.to_string());
            for (k, v) in &req.headers {
                r = r.header(k.as_str(), v.as_str());
            }
            let resp = r.send().await.map_err(|e| format!("The voice service didn't answer: {e}"))?;
            let status = resp.status();
            if !status.is_success() {
                let detail = resp.text().await.unwrap_or_default();
                // The provider's own words: "voice not found" and "quota exceeded"
                // need different actions, and a bare status code says neither.
                return Err(format!(
                    "The voice service refused ({status}): {}",
                    detail.chars().take(200).collect::<String>()
                ));
            }
            resp.bytes().await.map_err(|e| format!("The voice service's reply was unreadable: {e}"))
        })?;
        if bytes.is_empty() {
            return Err("The voice service returned no audio".into());
        }
        let out = std::env::temp_dir().join(format!(
            "su-tts-{}-{}.{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0),
            req.extension
        ));
        std::fs::write(&out, &bytes).map_err(|e| format!("Couldn't save the audio: {e}"))?;
        Ok(out)
    }

    /// Run a neural engine to produce a wav for one sentence.
    pub fn synthesize(&self, data_dir: &Path, engine: Engine, voice: &str, text: &str) -> Result<PathBuf, String> {
        let spec = spec_for(data_dir, engine)
            .ok_or_else(|| format!("no command configured for {}", engine.as_str()))?;
        if !binary_exists(&spec.binary) {
            return Err(format!(
                "{} needs `{}`, which isn't installed or isn't on this machine's PATH",
                engine.as_str(),
                spec.binary
            ));
        }
        let model = model_for(data_dir, engine, voice)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if engine == Engine::Piper && model.is_empty() {
            return Err(format!(
                "piper has no voice model — put a .onnx in {}",
                model_dir(data_dir, engine).display()
            ));
        }
        let out = std::env::temp_dir().join(format!(
            "su-tts-{}-{}.wav",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        let args = render_args(&spec, &model, voice, &out.to_string_lossy(), text);
        let mut cmd = Command::new(&spec.binary);
        cmd.args(&args).stdout(Stdio::null()).stderr(Stdio::piped());
        cmd.stdin(if spec.text_on_stdin { Stdio::piped() } else { Stdio::null() });
        let mut child = cmd.spawn().map_err(|e| format!("couldn't start `{}`: {e}", spec.binary))?;
        if spec.text_on_stdin {
            if let Some(mut si) = child.stdin.take() {
                let _ = si.write_all(text.as_bytes());
                // Dropping stdin is what tells the engine the sentence is complete;
                // without it these CLIs wait for more input and never finish.
            }
        }
        let done = child.wait_with_output().map_err(|e| format!("{} failed: {e}", spec.binary))?;
        if !done.status.success() {
            let err = String::from_utf8_lossy(&done.stderr);
            return Err(format!(
                "{} couldn't speak that: {}",
                spec.binary,
                err.lines().last().unwrap_or("no output").chars().take(160).collect::<String>()
            ));
        }
        if !out.exists() {
            return Err(format!("{} produced no audio", spec.binary));
        }
        Ok(out)
    }

    /// Stop immediately — this is barge-in. Killing mid-word is the point: a voice
    /// that finishes its sentence after you start talking is talking over you.
    pub fn stop(&self) {
        if let Some(mut c) = self.current.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if let Some(f) = self.temp.lock().unwrap().take() {
            let _ = std::fs::remove_file(f);
        }
    }

    /// Ask the whole answer to stop. Barge-in calls this, then `stop`.
    pub fn cancel_all(&self) {
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }

    /// Add a sentence to the back of the queue.
    pub fn enqueue(&self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            q.push_back(text.to_string());
        }
    }

    pub fn next_queued(&self) -> Option<String> {
        self.queue.lock().ok().and_then(|mut q| q.pop_front())
    }
}
