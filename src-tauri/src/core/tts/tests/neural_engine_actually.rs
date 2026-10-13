use super::*;

    pub(super) fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-tts-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(d.join("speech").join("piper")).unwrap();
        d
    }

    #[test]
    pub(super) fn piper_voices_are_read_from_the_models_on_disk() {
        // One .onnx per voice, so the filenames are the voice list.
        let d = scratch();
        let dir = model_dir(&d, Engine::Piper);
        std::fs::write(dir.join("en_GB-alan-medium.onnx"), b"x").unwrap();
        std::fs::write(dir.join("en_US-amy-low.onnx"), b"x").unwrap();
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();
        assert_eq!(
            installed_voices(&d, Engine::Piper),
            vec!["en_GB-alan-medium".to_string(), "en_US-amy-low".to_string()]
        );
    }

    #[test]
    pub(super) fn a_named_voice_wins_and_a_single_model_needs_no_naming() {
        let d = scratch();
        let dir = model_dir(&d, Engine::Piper);
        std::fs::write(dir.join("alan.onnx"), b"x").unwrap();
        std::fs::write(dir.join("amy.onnx"), b"x").unwrap();
        let picked = model_for(&d, Engine::Piper, "amy").unwrap();
        assert!(picked.ends_with("amy.onnx"));
        // Asking for nothing still resolves, rather than making the user name the
        // only voice they have.
        assert!(model_for(&d, Engine::Piper, "").is_some());
        // A voice that isn't installed falls back rather than pointing at a
        // file that does not exist.
        assert!(model_for(&d, Engine::Piper, "nobody").unwrap().exists());
    }

    #[test]
    pub(super) fn placeholders_are_filled_from_the_spec() {
        let spec = default_spec(Engine::Piper).unwrap();
        let args = render_args(&spec, "/m/alan.onnx", "alan", "/tmp/o.wav", "hello");
        assert_eq!(args, vec!["--model", "/m/alan.onnx", "--output_file", "/tmp/o.wav"]);
        assert!(spec.text_on_stdin, "piper reads its text on stdin");
    }

    #[test]
    pub(super) fn a_user_override_replaces_the_default_command() {
        // These CLIs change; a flag rename must be fixable without a rebuild.
        let d = scratch();
        let json = r#"{"piper":{"binary":"my-piper","args":["--voice","{voice}","-o","{out}"],"text_on_stdin":false}}"#;
        std::fs::write(speech_dir(&d).join("engines.json"), json).unwrap();
        let spec = spec_for(&d, Engine::Piper).unwrap();
        assert_eq!(spec.binary, "my-piper");
        assert!(!spec.text_on_stdin);
        assert_eq!(render_args(&spec, "m", "amy", "/tmp/o.wav", "hi"), vec!["--voice", "amy", "-o", "/tmp/o.wav"]);
    }

    #[test]
    pub(super) fn a_corrupt_override_falls_back_instead_of_breaking_speech() {
        let d = scratch();
        std::fs::write(speech_dir(&d).join("engines.json"), b"{ not json").unwrap();
        assert_eq!(spec_for(&d, Engine::Piper), default_spec(Engine::Piper));
    }

    #[test]
    pub(super) fn an_engine_that_is_not_installed_says_so_rather_than_looking_ready() {
        let d = scratch();
        let st = engine_status(&d, Engine::Piper);
        assert!(!st.available);
        let why = st.reason.unwrap();
        assert!(why.contains("piper"), "the message should name what to install: {why}");
    }

    #[test]
    pub(super) fn the_system_voice_is_always_available() {
        let d = scratch();
        let st = engine_status(&d, Engine::System);
        assert!(st.available && st.reason.is_none());
    }

    #[test]
    pub(super) fn kokoro_does_not_demand_a_model_file_per_voice() {
        // Its voices live inside one model, so an empty models folder is not the
        // reason it cannot run — only a missing binary is.
        let d = scratch();
        let st = engine_status(&d, Engine::Kokoro);
        let why = st.reason.unwrap_or_default();
        assert!(!why.contains("no voice models"), "kokoro was judged by piper's rule: {why}");
    }

    #[test]
    pub(super) fn playback_is_a_separate_process_so_it_can_be_killed() {
        let (program, args) = play_command("/tmp/a.wav");
        if cfg!(target_os = "macos") {
            assert_eq!(program, "afplay");
            assert_eq!(args, vec!["/tmp/a.wav"]);
        } else if cfg!(target_os = "windows") {
            assert_eq!(program, "powershell");
        } else {
            assert_eq!(program, "aplay");
        }
    }

    #[test]
    pub(super) fn a_missing_binary_is_never_silently_swapped_for_the_system_voice() {
        // The dishonesty this replaces: choosing piper appeared to work while the
        // system voice spoke instead, and nothing said so.
        let d = scratch();
        let s = Speaker::default();
        let err = s.speak(&d, Engine::Piper, "", "hello").unwrap_err();
        assert!(err.contains("piper"), "got: {err}");
    }

    #[test]
    pub(super) fn a_cloud_voice_with_no_key_says_which_key_is_missing() {
        // It used to answer "not implemented"; now it either speaks or explains
        // precisely what is missing, and never falls back to another voice.
        let d = scratch();
        std::fs::write(speech_dir(&d).join("cloud.json"), r#"{"provider":"elevenlabs","voice":"v","model":"m"}"#).unwrap();
        let s = Speaker::default();
        match s.fetch_cloud(&d, "hello") {
            Ok(f) => {
                // A key happens to be configured on this machine; then it must have
                // produced real audio rather than an empty file.
                assert!(std::fs::metadata(&f).unwrap().len() > 0);
                let _ = std::fs::remove_file(f);
            }
            Err(e) => assert!(e.contains("elevenlabs") && e.contains("key"), "got: {e}"),
        }
    }

    #[test]
    pub(super) fn a_neural_engine_is_actually_run_and_its_audio_is_used() {
        // Stands in for piper: same contract — read text on stdin, write a wav to
        // the path we asked for. Proves the spec, the placeholders, the stdin
        // hand-off and the "did it produce audio" check all line up, without
        // needing piper installed.
        let d = scratch();
        let fake = d.join("fake-engine.sh");
        std::fs::write(
            &fake,
            "#!/bin/sh\nout=\"\"\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in --output_file) out=\"$2\"; shift 2;; *) shift;; esac\ndone\ncat > /dev/null\nprintf 'RIFF....WAVEfmt ' > \"$out\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(model_dir(&d, Engine::Piper).join("alan.onnx"), b"x").unwrap();
        let spec = serde_json::json!({
            "piper": { "binary": fake.to_string_lossy(), "args": ["--model", "{model}", "--output_file", "{out}"], "text_on_stdin": true }
        });
        std::fs::write(speech_dir(&d).join("engines.json"), spec.to_string()).unwrap();

        let s = Speaker::default();
        let wav = s.synthesize(&d, Engine::Piper, "alan", "hello there").expect("synthesis should run");
        assert!(wav.exists(), "the engine's audio should be where we asked for it");
        assert!(std::fs::metadata(&wav).unwrap().len() > 0);
        let _ = std::fs::remove_file(wav);
    }
