use super::*;

/// Open the microphone and start collecting audio.
///
/// Samples are accumulated in memory rather than written to disk: a recording that
/// never lands in a file cannot be left behind by a crash, and dictation clips are
/// short by nature.
#[tauri::command]
pub fn start_dictation(state: tauri::State<AppState>) -> Result<DictationStatus, String> {
    if state.dictation.recording.load(Ordering::SeqCst) {
        return Ok(status_of(&state));
    }
    if !dictation::model_looks_complete(&state.data_dir) {
        return Err("The speech model isn't installed yet — set it up in Settings ▸ Voice input.".into());
    }
    let (device, _, reason) = describe_device_named(&chosen_mic(&state));
    let device = match (device, reason) {
        (Some(d), _) => d,
        (None, Some(why)) => return Err(why),
        (None, None) => return Err("No microphone is available.".into()),
    };
    let config = device.default_input_config().map_err(|e| format!("Couldn't read the microphone's format: {e}"))?;
    let channels = config.channels();
    let rate = config.sample_rate();

    state.dictation.samples.lock().unwrap().clear();
    state.dictation.source_rate.store(rate, Ordering::SeqCst);
    state.dictation.set_level(0.0);

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let samples = state.dictation.samples.clone();
    let level_milli = state.dictation.level_milli.clone();
    let thread_stop = stop.clone();
    let sample_format = config.sample_format();
    let stream_config = cpal::StreamConfig {
        channels,
        sample_rate: rate,
        buffer_size: cpal::BufferSize::Default,
    };

    // cpal's Stream is !Send, so it lives on its own thread and is dropped there.
    // Parking the thread on a flag is what lets `stop_dictation` end the capture
    // from the command thread without moving the stream across threads.
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::spawn(move || {
        // One sink, built fresh per sample format. Whatever the device hands back
        // is normalised to f32 here so nothing downstream has to know the format.
        macro_rules! sink {
            ($t:ty, $conv:expr) => {{
                let samples = samples.clone();
                let level_milli = level_milli.clone();
                let conv: fn(&$t) -> f32 = $conv;
                move |d: &[$t], _: &cpal::InputCallbackInfo| {
                    let f: Vec<f32> = d.iter().map(conv).collect();
                    level_milli.store((level_of(&f).clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
                    if let Ok(mut buf) = samples.lock() {
                        buf.extend_from_slice(&f);
                    }
                }
            }};
        }
        let err_fn = |e| eprintln!("[dictation] input stream error: {e}");
        let built = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(stream_config, sink!(f32, |s| *s), err_fn, None),
            cpal::SampleFormat::I16 => device.build_input_stream(stream_config, sink!(i16, |s| *s as f32 / i16::MAX as f32), err_fn, None),
            cpal::SampleFormat::I32 => device.build_input_stream(stream_config, sink!(i32, |s| *s as f32 / i32::MAX as f32), err_fn, None),
            cpal::SampleFormat::U16 => device.build_input_stream(stream_config, sink!(u16, |s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0), err_fn, None),
            other => {
                let _ = ready_tx.send(Err(format!("This microphone reports an audio format we can't read ({other:?}).")));
                return;
            }
        };
        let stream = match built {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("Couldn't open the microphone: {e}")));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(format!("Couldn't start the microphone: {e}")));
            return;
        }
        let _ = ready_tx.send(Ok(()));
        while !thread_stop.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        drop(stream);
    });

    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("The microphone didn't start in time.".into()),
    }

    *state.dictation.stream.lock().unwrap() = Some(Box::new(Capture { stop }));
    state.dictation.recording.store(true, Ordering::SeqCst);
    Ok(status_of(&state))
}

pub(super) fn end_capture(state: &AppState) -> (Vec<f32>, u32) {
    if let Some(h) = state.dictation.stream.lock().unwrap().take() {
        h.stop();
    }
    state.dictation.recording.store(false, Ordering::SeqCst);
    state.dictation.set_level(0.0);
    // Give the capture thread a moment to notice the flag and drop the stream.
    std::thread::sleep(std::time::Duration::from_millis(80));
    let samples = std::mem::take(&mut *state.dictation.samples.lock().unwrap());
    (samples, state.dictation.source_rate.load(Ordering::SeqCst))
}

/// Stop recording and transcribe what was captured.
#[tauri::command]
pub fn stop_dictation(state: tauri::State<AppState>) -> Result<String, String> {
    let (raw, rate) = end_capture(&state);
    let channels = {
        // The SAME device the capture came from. Reading the default's channel count
        // to de-interleave another device's audio is how a stereo USB mic turns into
        // half-speed noise.
        let (device, _, _) = describe_device_named(&chosen_mic(&state));
        device
            .and_then(|d| d.default_input_config().ok())
            .map(|c| c.channels())
            .unwrap_or(1)
    };
    let audio = to_whisper_mono(&raw, channels, rate);
    // Under a quarter second there is nothing to transcribe, and whisper will
    // hallucinate a sentence out of a click rather than return nothing.
    if audio.len() < (dictation::TARGET_RATE / 4) as usize {
        return Ok(String::new());
    }
    transcribe(&state.data_dir, &audio)
}

#[tauri::command]
pub fn cancel_dictation(state: tauri::State<AppState>) {
    let _ = end_capture(&state);
}

/// Run whisper over 16 kHz mono audio.
pub fn transcribe(data_dir: &std::path::Path, audio: &[f32]) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
    let model = dictation::model_path(data_dir);
    let ctx = WhisperContext::new_with_params(
        model.to_str().ok_or("the model path isn't valid text")?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("Couldn't load the speech model: {e}"))?;
    let mut st = ctx.create_state().map_err(|e| format!("Couldn't start transcription: {e}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_translate(false);
    params.set_language(Some("en"));
    // The progress/timestamp chatter goes to stdout otherwise, which in a desktop
    // app means it goes nowhere useful and slows the run down.
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    st.full(params, audio).map_err(|e| format!("Transcription failed: {e}"))?;
    let mut out = String::new();
    // `to_str_lossy` rather than `to_str`: a model can emit a byte sequence that
    // isn't valid UTF-8 mid-word, and losing one character beats losing the whole
    // transcript to an error.
    for seg in st.as_iter() {
        if let Ok(text) = seg.to_str_lossy() {
            out.push_str(&text);
            out.push(' ');
        }
    }
    Ok(dictation::clean_transcript(&out))
}
