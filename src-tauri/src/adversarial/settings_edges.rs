use crate::core::settings::Settings;

    #[test]
    fn hostile_numbers_do_not_produce_nonsense_budgets() {
        let mut s = Settings::default();
        for v in [0u64, 1, u32::MAX as u64, u64::MAX] {
            s.apply_patch(&serde_json::json!({
                "num_ctx": v, "max_tokens": v,
                "agent_max_context_tokens": v, "agent_max_context_turns": v, "agent_max_steps": v,
                "sync_interval_min": v, "subconscious_interval_min": v,
            }));
            for (name, got) in [
                ("num_ctx", s.num_ctx),
                ("max_tokens", s.max_tokens),
                ("agent_max_context_tokens", s.agent_max_context_tokens),
                ("agent_max_context_turns", s.agent_max_context_turns),
                ("agent_max_steps", s.agent_max_steps),
                ("sync_interval_min", s.sync_interval_min),
                ("subconscious_interval_min", s.subconscious_interval_min),
            ] {
                assert!(got > 0, "patching {name} with {v} produced {got} — a zero budget/interval");
            }
        }
    }

    #[test]
    fn a_corrupt_settings_file_does_not_silently_reset_privacy() {
        // load() parses with a default fallback. If the file is truncated, the
        // user's local_only choice must not quietly flip to the default.
        let d = super::scratch("settings-corrupt");
        let s = Settings { local_only: false, cloud_enabled: true, ..Default::default() };
        s.save(&d);
        let path = d.join("settings.json");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, &text[..text.len() / 2]).unwrap(); // truncated write
        let back = Settings::load(&d);
        let flipped = back.local_only != s.local_only;
        std::fs::remove_dir_all(&d).ok();
        assert!(!flipped, "a truncated settings.json reset local_only to the default instead of erroring");
    }
