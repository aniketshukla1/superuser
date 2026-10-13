use super::*;

        #[tokio::test]
        pub(super) async fn not_now_ends_the_wait_with_a_no_rather_than_a_timeout() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, "bt-help-no", "auto", false);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            ctx.events = Some(tx);
            let args = json!({"reason": "Sign in to example.com"});
            let asked = dispatch(&ctx, "browser_request_help", &args);
            let user = async {
                rx.recv().await.expect("the ask never reached the user");
                assert!(computer::existing("bt-help-no").unwrap().decline_help());
            };
            let (out, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async { tokio::join!(asked, user) })
                .await
                .expect("not now didn't end the wait");
            assert!(out.contains("not now") && !out.contains("in time"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn a_secret_for_a_field_that_was_never_seen_is_not_asked_for() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, "bt-secret", "auto", false);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            ctx.events = Some(tx);
            let out = dispatch(&ctx, "browser_request_secret", &json!({"label": "Password", "ref": "e1", "snapshot_id": 1})).await;
            assert!(out.starts_with("error:"), "got: {out}");
            assert!(rx.try_recv().is_err(), "the user was asked to type a secret into nothing");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        pub(super) fn a_boundary_keeps_its_first_word_and_everything_else_reads_as_an_error() {
            assert_eq!(browser_failed("Refused. Your browser was stopped from reaching x".into()), "Refused. Your browser was stopped from reaching x");
            assert_eq!(browser_failed("The browser closed.".into()), "error: The browser closed.");
        }

        #[test]
        pub(super) fn kept_screens_do_not_use_up_the_charts() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "auto", vec![]);
            for i in 0..20 {
                ctx.note_item(json!({"kind": "computer", "frame": format!("f{i}")}));
            }
            ctx.note_visual("chart", &json!({"spec": {}}));
            let kept = ctx.charts_drawn();
            assert_eq!(kept.iter().filter(|v| v["kind"] == "computer").count(), 12);
            assert_eq!(kept.iter().filter(|v| v["name"] == "chart").count(), 1);
            let _ = std::fs::remove_dir_all(&root);
        }
