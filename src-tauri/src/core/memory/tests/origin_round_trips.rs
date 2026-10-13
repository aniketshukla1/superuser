use super::*;

    #[test]
    pub(super) fn origin_round_trips_through_the_file() {
        let (eng, _g) = engine();
        let f = eng
            .save(&MemoryInput { title: "From Gmail".into(), body: "a mail body".into(), origin: Origin::External, ..Default::default() })
            .unwrap();
        assert_eq!(eng.parse_file(&eng.vault_path.join(&f)).unwrap().origin, Origin::External);

        let g = eng
            .save(&MemoryInput { title: "My note".into(), body: "I prefer terse replies".into(), origin: Origin::Internal, ..Default::default() })
            .unwrap();
        assert_eq!(eng.parse_file(&eng.vault_path.join(&g)).unwrap().origin, Origin::Internal);
    }

    #[test]
    pub(super) fn an_unmarked_memory_reads_as_external() {
        // Every memory written before this field existed, and anything a user drops
        // into the vault by hand. Guessing "internal" here would hand a pre-existing
        // vault full of synced email the trusted label.
        let (eng, g) = engine();
        std::fs::write(g.path.join("legacy.md"), "---\ntitle: \"Old\"\n---\n\nbody").unwrap();
        assert_eq!(eng.parse_file(&g.path.join("legacy.md")).unwrap().origin, Origin::External);
        // A garbage value is not a third state either.
        std::fs::write(g.path.join("weird.md"), "---\ntitle: \"Odd\"\norigin: \"trusted\"\n---\n\nbody").unwrap();
        assert_eq!(eng.parse_file(&g.path.join("weird.md")).unwrap().origin, Origin::External);
    }

    #[test]
    pub(super) fn reinforcing_an_external_memory_cannot_launder_it() {
        // Being recalled often is not evidence of trustworthiness.
        let (eng, _g) = engine();
        let f = eng
            .save(&MemoryInput { title: "Synced".into(), body: "from a connector".into(), origin: Origin::External, ..Default::default() })
            .unwrap();
        let m = eng.reinforce(&f).unwrap();
        assert_eq!(m.reinforce_count, 1);
        assert_eq!(m.origin, Origin::External);
    }

    #[test]
    pub(super) fn a_payload_that_omits_origin_deserializes_as_external() {
        // MemoryInput arrives from the window on the save path; trust has to be
        // claimed by the handler, never inherited from what the payload says.
        let input: MemoryInput = serde_json::from_str(r#"{"title":"x","body":"y"}"#).unwrap();
        assert_eq!(input.origin, Origin::External);
    }

    #[test]
    pub(super) fn decay_clamps_and_fresh_is_hot() {
        let (eng, _g) = engine();
        // very old last-interaction → clamped to the floor 0.12
        let old = eng.decay("2000-01-01T00:00:00+00:00", 1.0, 0);
        assert!((0.12..=0.13).contains(&old), "old weight was {old}");
        // fresh → near 1.0
        let now = chrono::Utc::now().to_rfc3339();
        assert!(eng.decay(&now, 1.0, 0) > 0.95);
    }
