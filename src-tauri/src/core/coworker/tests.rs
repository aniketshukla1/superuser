use super::*;
    use serde_json::json;

    fn scratch() -> std::path::PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-coworker-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn an_envelope_needs_a_task_and_says_who_asked() {
        assert!(Envelope::parse(&json!({"coworker": "analyst"})).unwrap_err().contains("what you are asking"));
        let e = Envelope::parse(&json!({"task": " Check the Q3 numbers ", "expecting": "a number"})).unwrap();
        assert_eq!((e.task.as_str(), e.constraints.as_str(), e.expecting.as_str()), ("Check the Q3 numbers", "", "a number"));
        let brief = e.brief("Assistant");
        assert!(brief.starts_with("Assistant has asked you to help"));
        assert!(brief.contains("Task: Check the Q3 numbers") && brief.contains("What a good answer looks like: a number"));
        assert!(!brief.contains("Constraints:"), "an empty field is left out, not sent empty");
        assert!(brief.contains("can't put a question to the person"));
        // The person's line carries the task and nothing meant for a model.
        assert_eq!(e.shown("Assistant"), "Assistant asked: Check the Q3 numbers");
    }

    #[test]
    fn the_same_ask_hashes_the_same_and_a_different_one_does_not() {
        let e = Envelope::parse(&json!({"task": "check the numbers"})).unwrap();
        let same = Envelope::parse(&json!({"task": "check the numbers"})).unwrap();
        let other = Envelope::parse(&json!({"task": "check the numbers", "constraints": "Q3 only"})).unwrap();
        assert_eq!(e.key("analyst"), same.key("analyst"));
        assert_ne!(e.key("analyst"), other.key("analyst"));
        assert_ne!(e.key("analyst"), e.key("finance"), "the same task to someone else is a different ask");
    }

    #[test]
    fn depth_is_read_off_the_conversation_and_nothing_else() {
        assert_eq!(depth_of("s-3a9f"), 0);
        assert_eq!(depth_of("ask1-91f0"), 1);
        assert_eq!(depth_of("ask2-91f0"), 2);
        // Not a hop id: no digits, or no separator after them.
        assert_eq!(depth_of("asked-something"), 0);
        assert_eq!(depth_of("ask12"), 0);
        assert!(session_id(1).starts_with("ask2-"));
    }

    #[test]
    fn the_caps_say_where_to_go_instead() {
        assert_eq!(cap_refusal(0, 0), None);
        assert_eq!(cap_refusal(1, 2), None);
        let deep = cap_refusal(MAX_DEPTH, 0).unwrap();
        assert!(deep.contains("as far as it goes") && deep.contains("what you have"));
        let wide = cap_refusal(0, MAX_PER_TURN).unwrap();
        assert!(wide.contains("as many as one turn may"));
    }

    #[test]
    fn a_hop_can_never_act_more_freely_than_the_run_that_asked() {
        assert_eq!(stricter("auto", "interactive"), "interactive");
        assert_eq!(stricter("interactive", "auto"), "interactive");
        assert_eq!(stricter("discuss", "auto"), "discuss");
        assert_eq!(stricter("auto", "auto"), "auto");
        // A mode nobody knows is treated as the strictest thing it could be.
        assert_eq!(stricter("auto", "something-new"), "something-new");
    }

    #[test]
    fn a_relayed_answer_carries_its_attribution_and_an_empty_one_is_not_dressed_up() {
        let out = relayed("Analyst", "  The figure is 41.2%.  ");
        assert!(out.contains("Analyst answered:") && out.contains("The figure is 41.2%.") && out.contains("came from Analyst"));
        assert!(relayed("Analyst", "   ").contains("finished without saying anything"));
        assert!(timed_out("Analyst").contains("five minutes"));
        assert!(failed("Analyst", "no model configured").contains("no model configured"));
        assert!(duplicate_refusal("Analyst").contains("already asked Analyst exactly this"));
    }

    #[test]
    fn a_roster_is_everyone_until_the_person_narrows_it() {
        let d = scratch();
        let mut reg = PersonaRegistry::load(&d);
        let everyone = may_ask(&reg, "assistant");
        assert!(everyone.iter().any(|c| c.id == "coder"), "every other assistant, until the person says otherwise");
        assert!(everyone.iter().all(|c| c.id != "assistant"), "never itself");
        let email = crate::core::email_bot::ASSISTANT;
        assert!(may_ask(&reg, email).is_empty(), "the Email Assistant reads strangers' mail, so it asks nobody unless granted");
        assert!(everyone.iter().any(|c| c.id == email), "it can still be asked — it only reads");
        reg.set_asks("assistant", vec!["coder".into(), "ops".into()]).unwrap();
        let reg = PersonaRegistry::load(&d);
        let roster = may_ask(&reg, "assistant");
        assert_eq!(roster.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["coder", "ops"]);
        let block = roster_block(&roster);
        assert!(block.contains("## Assistants you can ask") && block.contains("`coder`") && block.contains("`ops`"));
        assert!(roster_block(&[]).is_empty(), "no roster, no heading");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn who_gets_asked_is_resolved_by_id_first_then_name() {
        let d = scratch();
        let mut reg = PersonaRegistry::load(&d);
        reg.set_asks("assistant", vec!["coder".into()]).unwrap();
        let reg = PersonaRegistry::load(&d);
        assert_eq!(resolve(&reg, "assistant", "coder").unwrap().id, "coder");
        let by_name = reg.manifest("coder").unwrap().name.clone();
        assert_eq!(resolve(&reg, "assistant", &by_name.to_uppercase()).unwrap().id, "coder", "a name matches whatever its case");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn every_refusal_says_which_kind_of_no_it_is() {
        let d = scratch();
        let mut reg = PersonaRegistry::load(&d);
        reg.set_asks("assistant", vec!["coder".into()]).unwrap();
        let reg = PersonaRegistry::load(&d);
        // Itself — before anything about grants, whichever way the grants were set up.
        assert_eq!(resolve(&reg, "assistant", "assistant").unwrap_err(), SELF_REFUSAL);
        let own_name = reg.manifest("assistant").unwrap().name.clone();
        assert_eq!(resolve(&reg, "assistant", &own_name).unwrap_err(), SELF_REFUSAL);
        // Real, enabled, not granted: said in its own words, because it is a different
        // problem with a different fix.
        let ops = resolve(&reg, "assistant", "ops").unwrap_err();
        assert!(ops.contains("haven't been given") && ops.contains("who you may ask"), "{ops}");
        // Absent: the same words whether it never existed or is switched off, so a refusal
        // can't be used to find out what else is installed.
        assert!(resolve(&reg, "assistant", "no-such-assistant").unwrap_err().contains("There's no assistant called"));
        assert!(resolve(&reg, "assistant", "   ").unwrap_err().contains("name the assistant"));
        let _ = std::fs::remove_dir_all(&d);
    }

    // `@coder …` hands that one message to Coder — only when the mention opens the
    // message and names an assistant that is on and isn't the conversation's own.
    #[test]
    fn a_mention_opens_the_message_and_names_another_assistant_that_is_on() {
        let d = scratch();
        let mut reg = PersonaRegistry::load(&d);
        let (who, asked) = mentioned(&reg, "assistant", "@coder: why does the build fail?").unwrap();
        assert_eq!((who.id.as_str(), asked.as_str()), ("coder", "why does the build fail?"));
        assert_eq!(mentioned(&reg, "assistant", "  @CODER  ").unwrap().1, "", "case and padding don't matter");
        assert!(mentioned(&reg, "assistant", "I asked @coder earlier").is_none(), "it has to open the message");
        assert!(mentioned(&reg, "assistant", "@coders are busy").is_none(), "only a boundary ends the handle");
        assert!(mentioned(&reg, "assistant", "@assistant hello").is_none(), "that's just talking to it");
        assert!(mentioned(&reg, "assistant", "@brain what do you know?").is_none(), "the brain's, not an assistant's");
        assert!(mentioned(&reg, "assistant", "@nobody hi").is_none());
        reg.set_enabled("coder", false).unwrap();
        assert!(mentioned(&reg, "assistant", "@coder hi").is_none(), "switched off");

        let brief = mention_brief("Chief of Staff", "Expense Manager", "what did we spend?");
        assert!(brief.contains("talking with Chief of Staff") && brief.contains("to you, Expense Manager"));
        assert!(brief.ends_with("what did we spend?"), "the person's words come last, untouched");
        assert!(mention_brief("A", "B", "").contains("only called you in"), "a bare mention still gets an answer");
        let _ = std::fs::remove_dir_all(&d);
    }
