use super::*;

        #[tokio::test]
        async fn a_run_cannot_review_the_item_it_put_up() {
            // End to end, with the actor coming from ToolCtx rather than the args:
            // the model has no field it could fill in to become someone else.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let builder = ctx_for(&root, &memory, "interactive", vec![]);

            let added = dispatch(&builder, "board_add", &json!({
                "title": "Add the retry", "acceptance": "the flaky test passes ten times"
            })).await;
            let id = added.split('`').nth(1).expect("an id in backticks").to_string();

            dispatch(&builder, "board_move", &json!({ "id": id, "state": "review" })).await;
            let self_review = dispatch(&builder, "board_review", &json!({
                "id": id, "pass": true, "evidence": "I ran them"
            })).await;
            assert!(self_review.starts_with("error:"), "got: {self_review}");
            assert!(self_review.contains("cannot be the one to pass it"), "got: {self_review}");

            let premature = dispatch(&builder, "board_move", &json!({ "id": id, "state": "done" })).await;
            assert!(premature.starts_with("error:"), "got: {premature}");

            // A different run — what a spawned subagent gets — can.
            let mut verifier = ctx_for(&root, &memory, "interactive", vec![]);
            verifier.session_id = format!("{}-sub2", builder.session_id);
            let ok = dispatch(&verifier, "board_review", &json!({
                "id": id, "pass": true, "evidence": "ran the suite ten times, green"
            })).await;
            assert!(!ok.starts_with("error:"), "got: {ok}");

            let done = dispatch(&builder, "board_move", &json!({ "id": id, "state": "done" })).await;
            assert!(!done.starts_with("error:"), "got: {done}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn the_board_reads_back_what_is_waiting() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            assert!(dispatch(&ctx, "board_list", &json!({})).await.contains("empty"));

            dispatch(&ctx, "board_add", &json!({ "title": "Ship it", "acceptance": "tests green" })).await;
            let text = dispatch(&ctx, "board_list", &json!({})).await;
            assert!(text.contains("Ship it"), "{text}");
            assert!(text.contains("done when: tests green"), "{text}");

            let bad = dispatch(&ctx, "board_move", &json!({ "id": "nope", "state": "done" })).await;
            assert!(bad.contains("no item"), "got: {bad}");
            let _ = std::fs::remove_dir_all(&root);
        }
