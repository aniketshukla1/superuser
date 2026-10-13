use super::*;

    /// The whole path, on real audio: a spoken clip in, words out.
    ///
    /// Ignored by default because it needs the ~148 MB model on disk — the unit
    /// tests around it cover the signal maths without it. Run with:
    ///   SU_DATA_DIR=… SU_WAV=… cargo test --no-default-features -- --ignored hears
    #[test]
    #[ignore]
    fn hears_what_was_actually_said() {
        let data_dir = std::path::PathBuf::from(std::env::var("SU_DATA_DIR").expect("SU_DATA_DIR"));
        let wav = std::env::var("SU_WAV").expect("SU_WAV");
        let mut reader = hound::WavReader::open(&wav).expect("open wav");
        let spec = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i16>()
                .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
                .collect(),
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        };
        let audio = to_whisper_mono(&samples, spec.channels, spec.sample_rate);
        let text = transcribe(&data_dir, &audio).expect("transcription should succeed");
        println!("TRANSCRIPT: {text}");
        let lower = text.to_lowercase();
        assert!(lower.contains("meeting room"), "expected the spoken words back, got: {text}");
        assert!(lower.contains("thursday"), "expected the spoken words back, got: {text}");
    }
