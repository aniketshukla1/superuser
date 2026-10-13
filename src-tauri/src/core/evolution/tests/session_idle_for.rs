use super::*;

    pub(super) fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-evo-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// A session `mins` minutes idle with `turns` user turns since the last pass.
    pub(super) fn session_idle_for(mins: f64, turns: usize) -> crate::core::sessions::Session {
        use crate::core::sessions::{Message, Session};
        let now = 1_000_000.0;
        let mut s = Session::new("assistant", "/tmp/ws", "llama3.2");
        for i in 0..turns {
            let mut u = Message::new("user", format!("q{i}"));
            u.at = now - mins * 60.0;
            let mut a = Message::new("assistant", format!("a{i}"));
            a.at = now - mins * 60.0;
            s.messages.push(u);
            s.messages.push(a);
        }
        s.updated_at = now - mins * 60.0;
        s
    }

    pub(super) const NOW: f64 = 1_000_000.0;

    #[test]
    pub(super) fn a_conversation_still_in_progress_is_never_reviewed() {
        // Plenty of material, but the user typed a minute ago — reviewing now would
        // interrupt someone who is still thinking.
        let s = session_idle_for(1.0, 20);
        assert_eq!(due(&s, 0, 48_000, 10, 6, NOW), Due::NotIdleYet);
        assert!(!due(&s, 0, 48_000, 10, 6, NOW).should_run());
    }

    #[test]
    pub(super) fn an_idle_conversation_with_enough_turns_is_reviewed() {
        let s = session_idle_for(30.0, 6);
        assert_eq!(due(&s, 0, 48_000, 10, 6, NOW), Due::Turns);
    }

    #[test]
    pub(super) fn a_short_idle_conversation_is_left_alone() {
        // Idle, but two turns of small talk are not worth a model call.
        let s = session_idle_for(30.0, 2);
        assert_eq!(due(&s, 0, 48_000, 10, 6, NOW), Due::NothingNewToReview);
        assert_eq!(due(&session_idle_for(30.0, 0), 0, 48_000, 10, 6, NOW), Due::Empty);
    }

    #[test]
    pub(super) fn context_pressure_triggers_a_review_before_the_turn_threshold() {
        // Few turns but a huge conversation: it is about to start losing its early
        // turns to trimming, so review it while the material still exists.
        let s = session_idle_for(30.0, 2);
        assert_eq!(due(&s, 40_000, 48_000, 10, 6, NOW), Due::ContextPressure);
        assert_eq!(due(&s, 20_000, 48_000, 10, 6, NOW), Due::NothingNewToReview, "half full is not pressure");
    }

    #[test]
    pub(super) fn turns_are_counted_from_the_last_pass_not_the_start() {
        let mut s = session_idle_for(30.0, 10);
        assert_eq!(s.turns_since_evolution(), 10);
        // A pass just ran: everything before it is already reviewed, so the same
        // session must not immediately qualify again.
        s.evolved_at = NOW;
        assert_eq!(s.turns_since_evolution(), 0);
        assert_eq!(due(&s, 0, 48_000, 10, 6, NOW), Due::NothingNewToReview);
    }

    #[test]
    pub(super) fn the_transcript_is_only_what_was_said() {
        use crate::core::sessions::{Message, Session};
        let mut s = Session::new("assistant", "/tmp/ws", "llama3.2");
        s.messages.push(Message::new("user", "hello"));
        s.messages.push(Message::new("assistant", "hi"));
        s.messages.push(Message::new("system", "internal bookkeeping"));
        s.messages.push(Message::new("assistant", "   "));
        let t = s.transcript();
        assert!(t.contains("user: hello"));
        assert!(t.contains("assistant: hi"));
        assert!(!t.contains("internal bookkeeping"), "system chatter is not something to learn from");
        assert_eq!(t.matches("assistant:").count(), 1, "blank turns are skipped");
    }

    #[test]
    pub(super) fn a_silent_review_parses_to_nothing() {
        assert!(parse_review("<silent/>").is_none());
        assert!(parse_review("Nothing here warrants a change.\n<silent/>").is_none());
        // No edit or pattern blocks at all is also "change nothing", however chatty.
        assert!(parse_review("I considered updating the skill but decided against it.").is_none());
        assert!(parse_review("").is_none());
    }

    #[test]
    pub(super) fn edits_and_the_note_are_parsed() {
        let raw = "<edit path=\"skills/release/SKILL.md\">\n---\nname: release\n---\nSteps here\n</edit>\n<note>I fixed the release skill's missing tag step.</note>";
        let r = parse_review(raw).unwrap();
        assert_eq!(r.edits.len(), 1);
        assert_eq!(r.edits[0].path, "skills/release/SKILL.md");
        assert!(r.edits[0].content.starts_with("---\nname: release"));
        assert!(r.edits[0].content.contains("Steps here"));
        assert_eq!(r.note, "I fixed the release skill's missing tag step.");
    }

    #[test]
    pub(super) fn several_edits_in_one_pass_are_all_parsed() {
        let raw = "<edit path=\"skills/a/SKILL.md\">one</edit><edit path=\"journal/MEMORY.md\">- two</edit><note>did both</note>";
        let r = parse_review(raw).unwrap();
        assert_eq!(r.edits.len(), 2);
        assert_eq!(r.edits[1].path, "journal/MEMORY.md");
        assert_eq!(r.note, "did both");
    }

    #[test]
    pub(super) fn a_pattern_on_its_own_is_a_real_answer_not_a_silent_one() {
        // The change that makes the layer work: a pass that noticed something but had
        // no skill fix ready used to parse to nothing and be thrown away.
        let raw = "<pattern id=\"changelog-step\" title=\"Release skips the changelog\" kind=\"failure\">\nThe release skill never tags the changelog, so 0.4.1 shipped without one.\n</pattern>";
        let r = parse_review(raw).unwrap();
        assert!(r.edits.is_empty(), "no file changed");
        assert_eq!(r.patterns.len(), 1);
        assert_eq!(r.patterns[0].id, "changelog-step");
        assert_eq!(r.patterns[0].title, "Release skips the changelog");
        assert_eq!(r.patterns[0].kind, wiki::Kind::Failure);
        assert!(r.patterns[0].body.starts_with("The release skill"));
    }

    #[test]
    pub(super) fn a_strategy_is_not_recorded_as_a_failure() {
        let r = parse_review("<pattern id=\"ask-first\" title=\"Asking first worked\" kind=\"strategy\">body</pattern>").unwrap();
        assert_eq!(r.patterns[0].kind, wiki::Kind::Strategy);
        // A missing or mistyped kind reads as a failure rather than being dropped.
        let r = parse_review("<pattern id=\"x\" title=\"y\">body</pattern>").unwrap();
        assert_eq!(r.patterns[0].kind, wiki::Kind::Failure);
    }

    #[test]
    pub(super) fn a_pattern_with_no_id_is_dropped_because_it_could_never_be_reinforced() {
        // An id is the only thing that lets the second sighting find the first.
        assert!(parse_review("<pattern title=\"nameless\">body</pattern>").is_none());
        assert!(parse_review("<pattern id=\"  \" title=\"blank\">body</pattern>").is_none());
        assert!(parse_review("<pattern id=\"ok\">   </pattern>").is_none(), "an empty body is not a pattern");
        assert!(parse_review("<pattern id=\"ok\">truncated mid-pat").is_none());
    }

    #[test]
    pub(super) fn a_pattern_id_from_the_model_is_slugified_before_it_becomes_a_filename() {
        let r = parse_review("<pattern id=\"../../etc/passwd\" title=\"t\">body</pattern>").unwrap();
        assert!(!r.patterns[0].id.contains(".."), "got {:?}", r.patterns[0].id);
        assert!(!r.patterns[0].id.contains('/'));
    }

    #[test]
    pub(super) fn an_edit_can_cite_the_patterns_that_motivated_it() {
        let raw = "<edit path=\"skills/release/SKILL.md\">---\nname: release\n---\nbody</edit>\n                   <because>changelog-step, tag-order</because><note>Fixed it.</note>";
        let r = parse_review(raw).unwrap();
        assert_eq!(r.because, ["changelog-step", "tag-order"]);
        assert_eq!(r.note, "Fixed it.");
        assert!(parse_review("<edit path=\"skills/a/SKILL.md\">x</edit>").unwrap().because.is_empty());
    }

    #[test]
    pub(super) fn edits_and_patterns_in_one_answer_are_both_taken() {
        let raw = "<pattern id=\"p\" title=\"T\" kind=\"failure\">why</pattern>\
                   <edit path=\"skills/a/SKILL.md\">---\nname: a\n---\nbody</edit><note>n</note>";
        let r = parse_review(raw).unwrap();
        assert_eq!(r.patterns.len(), 1);
        assert_eq!(r.edits.len(), 1);
    }
