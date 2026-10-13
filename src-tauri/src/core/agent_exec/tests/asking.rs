use super::*;

        fn asking_ctx<'a>(
            root: &PathBuf,
            memory: &'a crate::core::memory::MemoryEngine,
            sid: &str,
            events: Option<EventSink>,
            unattended: bool,
        ) -> ToolCtx<'a> {
            let mut c = ctx_for(root, memory, "interactive", vec![]);
            c.session_id = sid.into();
            c.events = events;
            c.unattended = unattended;
            c
        }

        fn a_real_question() -> Value {
            json!({ "questions": [{
                "question": "Which quarter should this cover?",
                "options": [
                    { "label": "Q3", "description": "The closed quarter — the numbers are final." },
                    { "label": "Q4", "description": "Still running; figures move until the 31st." }
                ]
            }]})
        }

        #[tokio::test]
        async fn the_answer_comes_back_as_the_tools_result() {
            // The whole point. Before this, `ask_user` returned "[awaiting user] …"
            // and the loop carried on — so asking cost a step and bought nothing,
            // and guessing was strictly the better move.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = asking_ctx(&root, &memory, "ask-roundtrip", Some(tx), false);

            let sid = ctx.session_id.clone();
            tokio::spawn(async move {
                for _ in 0..200 {
                    if crate::core::ask::is_waiting(&sid) {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                crate::core::ask::answer(&sid, r#"{"answers":["Q4"]}"#);
            });

            let out = dispatch(&ctx, "ask_user", &a_real_question()).await;
            assert!(out.contains("Q4"), "the user's answer is the result: {out}");

            // …and the question reached the GUI on the way, in the shape its card reads.
            let frame = rx.recv().await.expect("the question has to be sent before it can be answered");
            assert_eq!(frame["type"], "question");
            assert_eq!(frame["questions"][0]["question"], "Which quarter should this cover?");
            assert_eq!(frame["questions"][0]["options"][1]["label"], "Q4");
            assert_eq!(frame["questions"][0]["options"][1]["description"], "Still running; figures move until the 31st.");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_question_nobody_can_hear_is_refused_rather_than_awaited() {
            // No socket attached: waiting would hang the turn on an answer that
            // cannot arrive.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = asking_ctx(&root, &memory, "ask-nosink", None, false);
            let out = dispatch(&ctx, "ask_user", &a_real_question()).await;
            assert!(out.starts_with("error:"), "got: {out}");
            assert!(!crate::core::ask::is_waiting("ask-nosink"), "nothing should be left parked");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn an_unattended_run_parks_the_question_and_is_told_not_to_guess() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = asking_ctx(&root, &memory, "ask-unattended", Some(tx), true);

            let out = dispatch(&ctx, "ask_user", &a_real_question()).await;
            assert!(out.contains("Inbox"), "got: {out}");
            assert!(out.contains("Do not guess"), "an unattended guess acts unsupervised: {out}");
            assert!(!crate::core::ask::is_waiting("ask-unattended"), "an unattended run must not block");

            let parked = crate::core::inbox::Inbox::load(&root).pending();
            assert_eq!(parked.len(), 1);
            assert_eq!(parked[0].tool, "ask_user");
            assert_eq!(parked[0].summary, "Which quarter should this cover?");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_question_with_no_likely_answers_never_reaches_the_user() {
            // "Which sheet?" is the shape this whole card was rebuilt to stop: a fact
            // the agent could have looked up, handed over as a blank box.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = asking_ctx(&root, &memory, "ask-bare", Some(tx), false);
            let out = dispatch(&ctx, "ask_user", &json!({ "question": "Which sheet?" })).await;
            assert!(out.starts_with("error:"), "got: {out}");
            assert!(rx.try_recv().is_err(), "a blank-box question must not raise a card");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn an_empty_question_never_reaches_the_user() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = asking_ctx(&root, &memory, "ask-empty", Some(tx), false);
            let out = dispatch(&ctx, "ask_user", &json!({ "question": "  " })).await;
            assert!(out.starts_with("error:"), "got: {out}");
            assert!(rx.try_recv().is_err(), "an empty question must not raise a card");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_torn_down_turn_ends_the_wait_without_inventing_an_answer() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = asking_ctx(&root, &memory, "ask-abandoned", Some(tx), false);

            let sid = ctx.session_id.clone();
            tokio::spawn(async move {
                for _ in 0..200 {
                    if crate::core::ask::is_waiting(&sid) {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                crate::core::ask::abandon(&sid);
            });

            let out = dispatch(&ctx, "ask_user", &a_real_question()).await;
            assert!(out.contains("No answer"), "got: {out}");
            assert!(out.contains("Do not assume"), "silence must not become a yes: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }
