use super::*;

    #[test]
    pub(super) fn an_engine_that_writes_no_audio_is_reported_rather_than_played() {
        // A CLI that exits 0 having produced nothing would otherwise become a
        // silent "successful" sentence.
        let d = scratch();
        let fake = d.join("silent.sh");
        std::fs::write(&fake, "#!/bin/sh\ncat > /dev/null\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(model_dir(&d, Engine::Piper).join("alan.onnx"), b"x").unwrap();
        let spec = serde_json::json!({
            "piper": { "binary": fake.to_string_lossy(), "args": ["--output_file", "{out}"], "text_on_stdin": true }
        });
        std::fs::write(speech_dir(&d).join("engines.json"), spec.to_string()).unwrap();
        let s = Speaker::default();
        let err = s.synthesize(&d, Engine::Piper, "alan", "hi").unwrap_err();
        assert!(err.contains("no audio"), "got: {err}");
    }

    #[test]
    pub(super) fn an_engine_that_fails_reports_its_own_complaint() {
        let d = scratch();
        let fake = d.join("angry.sh");
        std::fs::write(&fake, "#!/bin/sh\ncat > /dev/null\necho 'unknown voice: bogus' >&2\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(model_dir(&d, Engine::Piper).join("alan.onnx"), b"x").unwrap();
        let spec = serde_json::json!({
            "piper": { "binary": fake.to_string_lossy(), "args": ["--output_file", "{out}"], "text_on_stdin": true }
        });
        std::fs::write(speech_dir(&d).join("engines.json"), spec.to_string()).unwrap();
        let s = Speaker::default();
        let err = s.synthesize(&d, Engine::Piper, "alan", "hi").unwrap_err();
        assert!(err.contains("unknown voice"), "the engine's own message should reach the user: {err}");
    }

    /// The real engine, not a stand-in.
    ///
    /// Ignored by default because it needs piper installed and a voice model on
    /// disk; the stand-in tests above cover the contract without either. Run with:
    ///   SU_DATA_DIR=… cargo test --no-default-features -- --ignored speaks_through_real_piper
    #[test]
    #[ignore]
    pub(super) fn speaks_through_real_piper() {
        let data_dir = std::path::PathBuf::from(std::env::var("SU_DATA_DIR").expect("SU_DATA_DIR"));
        let st = engine_status(&data_dir, Engine::Piper);
        assert!(st.available, "piper not ready: {:?}", st.reason);
        assert!(!st.voices.is_empty(), "no voice models installed");

        let s = Speaker::default();
        let wav = s
            .synthesize(&data_dir, Engine::Piper, &st.voices[0], "Piper is speaking through the real engine.")
            .expect("synthesis should run");
        let bytes = std::fs::metadata(&wav).expect("the wav should exist").len();
        assert!(bytes > 1000, "piper produced only {bytes} bytes");
        let _ = std::fs::remove_file(wav);
    }

    #[test]
    pub(super) fn openai_reuses_the_key_already_given_for_models() {
        // Asking for a second key for the same account would be a needless step.
        let cfg = CloudVoice::default();
        assert_eq!(cfg.provider, "openai");
        let req = cloud_request(&cfg, "sk-test", "hello").unwrap();
        assert_eq!(req.url, "https://api.openai.com/v1/audio/speech");
        assert!(req.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer sk-test"));
        assert_eq!(req.body["input"], "hello");
        assert_eq!(req.body["voice"], "alloy");
        // wav, so one player path covers cloud and local alike.
        assert_eq!(req.body["response_format"], "wav");
        assert_eq!(req.extension, "wav");
    }

    #[test]
    pub(super) fn elevenlabs_authenticates_its_own_way() {
        // A bearer token here would be silently rejected — it wants xi-api-key.
        let cfg = CloudVoice { provider: "elevenlabs".into(), voice: "abc123".into(), model: "eleven_turbo_v2_5".into() };
        let req = cloud_request(&cfg, "el-key", "hello").unwrap();
        assert!(req.url.contains("/v1/text-to-speech/abc123"), "voice id belongs in the path: {}", req.url);
        assert!(req.headers.iter().any(|(k, v)| k == "xi-api-key" && v == "el-key"));
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
        assert_eq!(req.body["model_id"], "eleven_turbo_v2_5");
        assert_eq!(req.extension, "mp3");
    }

    #[test]
    pub(super) fn elevenlabs_without_a_voice_id_says_so_instead_of_calling_a_bad_url() {
        let cfg = CloudVoice { provider: "elevenlabs".into(), voice: "  ".into(), model: "m".into() };
        assert!(cloud_request(&cfg, "k", "hi").unwrap_err().contains("voice id"));
    }

    #[test]
    pub(super) fn an_unknown_provider_is_refused_rather_than_guessed_at() {
        let cfg = CloudVoice { provider: "acme-voice".into(), voice: "v".into(), model: "m".into() };
        let err = cloud_request(&cfg, "k", "hi").unwrap_err();
        assert!(err.contains("acme-voice") && err.contains("openai"), "got: {err}");
    }

    #[test]
    pub(super) fn a_cloud_voice_with_no_key_reports_that_rather_than_looking_ready() {
        let d = scratch();
        std::fs::write(speech_dir(&d).join("cloud.json"), r#"{"provider":"elevenlabs","voice":"v","model":"m"}"#).unwrap();
        let st = engine_status(&d, Engine::Cloud);
        // The machine running tests has no ElevenLabs key, so this is the real path.
        if !st.available {
            assert!(st.reason.unwrap().contains("API key"));
        }
        assert_eq!(st.binary, "elevenlabs", "the status should name the provider");
    }

    #[test]
    pub(super) fn the_cloud_config_falls_back_rather_than_failing() {
        let d = scratch();
        std::fs::write(speech_dir(&d).join("cloud.json"), b"{ not json").unwrap();
        assert_eq!(cloud_config(&d), CloudVoice::default());
    }

    #[test]
    pub(super) fn a_chosen_cloud_voice_is_honoured() {
        let d = scratch();
        std::fs::write(speech_dir(&d).join("cloud.json"), r#"{"provider":"openai","voice":"nova","model":"tts-1-hd"}"#).unwrap();
        let cfg = cloud_config(&d);
        let req = cloud_request(&cfg, "k", "hi").unwrap();
        assert_eq!(req.body["voice"], "nova");
        assert_eq!(req.body["model"], "tts-1-hd");
    }

    #[test]
    pub(super) fn sentences_queue_rather_than_replacing_each_other() {
        // The failure this prevents: an answer arrives sentence by sentence, each
        // call cancels the last, and the user hears only the final sentence.
        let s = Speaker::default();
        s.enqueue("One.");
        s.enqueue("Two.");
        s.enqueue("Three.");
        assert_eq!(s.queue_len(), 3);
        assert_eq!(s.next_queued().as_deref(), Some("One."));
        assert_eq!(s.next_queued().as_deref(), Some("Two."));
    }

    #[test]
    pub(super) fn only_one_worker_drains_the_queue() {
        // Two speak() calls in the same breath must not become two voices.
        let s = Speaker::default();
        assert!(s.claim_drain(), "the first caller takes the job");
        assert!(!s.claim_drain(), "the second must not");
        s.release_drain();
        assert!(s.claim_drain(), "and it is available again afterwards");
    }

    #[test]
    pub(super) fn barge_in_empties_what_was_still_queued() {
        // Stopping the sentence in flight is useless if the next one starts anyway.
        let s = Speaker::default();
        s.enqueue("One.");
        s.enqueue("Two.");
        s.cancel_all();
        assert_eq!(s.queue_len(), 0);
        assert!(s.cancelled());
    }

    #[test]
    pub(super) fn empty_text_never_reaches_the_queue() {
        let s = Speaker::default();
        s.enqueue("   ");
        s.enqueue("");
        assert_eq!(s.queue_len(), 0);
    }
