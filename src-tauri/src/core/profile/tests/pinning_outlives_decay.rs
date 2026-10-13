use super::*;

    pub(super) fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-profile-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    pub(super) fn obs(class: Class, key: &str, value: &str, cue: Cue, at: f64) -> Observation {
        Observation { class, key: key.into(), value: value.into(), cue, at, evidence: value.into(), source: "s-1".into() }
    }

    #[test]
    pub(super) fn save_load_roundtrip() {
        let d = tmp();
        let p = Profile { name: "Jordan".into(), tone: "concise".into(), ..Default::default() };
        save(&d, &p);
        let got = load(&d);
        assert_eq!(got.name, "Jordan");
        assert_eq!(got.tone, "concise");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn empty_profile_yields_no_system_block() {
        let d = tmp();
        assert!(Learned::load(&d).system_block().is_empty());
        assert!(Profile::default().is_empty());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn the_form_still_reaches_the_prompt() {
        // The form was the whole profile before this module learned anything. An
        // install that never says another word must behave exactly as it did.
        let d = tmp();
        save(&d, &Profile { name: "Jordan Lee".into(), call_me: "Jordan".into(), tone: "concise".into(), dislikes: "fluff".into(), ..Default::default() });
        let block = Learned::load(&d).system_block();
        assert!(block.contains("Jordan"), "{block}");
        assert!(!block.contains("Jordan Lee"), "should prefer the call_me name: {block}");
        assert!(block.contains("concise"), "{block}");
        assert!(block.contains("fluff"), "{block}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_form_field_applies_without_earning_it_and_never_ages() {
        // This was Active by accident before it was Active by rule: the form's
        // observations are stamped `now`, so they only cleared the threshold while
        // their timestamp happened to be newer than the clock the scorer read.
        // Reorder those two lines and every profile in existence would have gone
        // quiet while still showing on screen.
        let d = tmp();
        save(&d, &Profile { call_me: "Nik".into(), ..Default::default() });
        let l = Learned::load(&d);
        // Scored a year in the future: a declaration doesn't age.
        let f = l.facets_at(now() + 365.0 * DAY);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].state, State::Active);
        assert!(f[0].from_form);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn saying_otherwise_beats_a_stale_form_field() {
        // The form applies, but it is not the last word — otherwise this module
        // would have re-frozen the profile it exists to unfreeze.
        let d = tmp();
        save(&d, &Profile { call_me: "Jordan".into(), ..Default::default() });
        let mut l = Learned::load(&d);
        l.observe(vec![obs(Class::Identity, "name", "Goes by Nik", Cue::Explicit, now())]);
        let f = l.facets();
        assert_eq!(f.iter().find(|f| f.value == "Goes by Nik").unwrap().state, State::Active);
        assert_eq!(f.iter().find(|f| f.value == "Jordan").unwrap().state, State::Provisional);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn one_explicit_statement_is_enough_but_one_guess_is_not() {
        // The threshold exists to keep inference out of the prompt until it has
        // repeated itself, without making the user say something twice.
        let d = tmp();
        let mut l = Learned::load(&d);
        l.observe(vec![
            obs(Class::Style, "reply-length", "Wants short answers", Cue::Explicit, now()),
            obs(Class::Tooling, "editor", "Uses Zed", Cue::Behavioral, now()),
        ]);
        let f = l.facets();
        let short = f.iter().find(|f| f.key == "reply-length").unwrap();
        let zed = f.iter().find(|f| f.key == "editor").unwrap();
        assert_eq!(short.state, State::Active);
        assert_eq!(zed.state, State::Provisional, "a single inference should not steer answers");

        // Seeing the same thing again is what promotes it.
        l.observe(vec![obs(Class::Tooling, "editor", "Uses Zed", Cue::Behavioral, now())]);
        let zed = l.facets().into_iter().find(|f| f.key == "editor").unwrap();
        assert_eq!(zed.state, State::Active);
        assert_eq!(zed.observations, 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn evidence_fades_at_the_classs_own_rate() {
        // Same statement, same age, different class: who someone is outlives how
        // they wanted a paragraph written two months ago.
        let d = tmp();
        let mut l = Learned::load(&d);
        let sixty_days_ago = now() - 60.0 * DAY;
        l.observe(vec![
            obs(Class::Identity, "role", "Works in infrastructure", Cue::Explicit, sixty_days_ago),
            obs(Class::Style, "reply-length", "Wants short answers", Cue::Explicit, sixty_days_ago),
        ]);
        let f = l.facets();
        assert_eq!(f.iter().find(|f| f.key == "role").unwrap().state, State::Active);
        assert_eq!(
            f.iter().find(|f| f.key == "reply-length").unwrap().state,
            State::Provisional,
            "a two-month-old style note should have faded"
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_beaten_value_can_win_the_key_back() {
        // The lifecycle claim, tested as behaviour: losing is not deletion.
        let d = tmp();
        let mut l = Learned::load(&d);
        l.observe(vec![obs(Class::Style, "tone", "Wants a formal tone", Cue::Explicit, now())]);
        l.observe(vec![obs(Class::Style, "tone", "Wants a casual tone", Cue::Behavioral, now())]);
        let f = l.facets();
        assert_eq!(f.iter().find(|f| f.value.contains("formal")).unwrap().state, State::Active);
        assert_eq!(f.iter().find(|f| f.value.contains("casual")).unwrap().state, State::Provisional);

        // Said outright twice, the newcomer overtakes it.
        l.observe(vec![
            obs(Class::Style, "tone", "Wants a casual tone", Cue::Explicit, now()),
            obs(Class::Style, "tone", "Wants a casual tone", Cue::Explicit, now()),
        ]);
        let f = l.facets();
        assert_eq!(f.iter().find(|f| f.value.contains("casual")).unwrap().state, State::Active);
        assert_eq!(f.iter().find(|f| f.value.contains("formal")).unwrap().state, State::Provisional);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn pinning_outlives_decay_and_forgetting_blocks_the_return() {
        let d = tmp();
        let mut l = Learned::load(&d);
        let ancient = now() - 400.0 * DAY;
        l.observe(vec![obs(Class::Style, "sign-off", "Never signs off with an em dash", Cue::Explicit, ancient)]);
        assert_eq!(l.facets()[0].state, State::Provisional, "ancient evidence should have faded");

        l.set_pinned(Class::Style, "sign-off", "Never signs off with an em dash", true);
        let f = &l.facets()[0];
        assert_eq!(f.state, State::Active);
        assert!(f.pinned);

        l.forget(Class::Style, "sign-off", "Never signs off with an em dash");
        assert!(l.facets().is_empty());
        // And it stays gone even if the model notices it again.
        l.observe(vec![obs(Class::Style, "sign-off", "Never signs off with an em dash", Cue::Explicit, now())]);
        assert!(l.facets().is_empty(), "a forgotten belief must not come straight back");

        l.unforget(Class::Style, "sign-off", "Never signs off with an em dash");
        l.observe(vec![obs(Class::Style, "sign-off", "Never signs off with an em dash", Cue::Explicit, now())]);
        assert_eq!(l.facets().len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_class_cannot_spend_another_classs_budget() {
        let d = tmp();
        let mut l = Learned::load(&d);
        for i in 0..12 {
            l.observe(vec![obs(Class::Style, &format!("k{i}"), &format!("Style rule {i}"), Cue::Explicit, now())]);
        }
        l.observe(vec![obs(Class::Identity, "role", "Works in infrastructure", Cue::Explicit, now())]);
        let active: Vec<Facet> = l.facets().into_iter().filter(|f| f.state == State::Active).collect();
        assert_eq!(active.iter().filter(|f| f.class == Class::Style).count(), Class::Style.budget());
        assert_eq!(active.iter().filter(|f| f.class == Class::Identity).count(), 1);
        std::fs::remove_dir_all(&d).ok();
    }
