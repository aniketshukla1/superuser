use super::*;
        use crate::core::cmdgate::grants;

        /// A ctx with a session id of its very own.
        ///
        /// Grants live in a process-global keyed by session id, and `ctx_for` hands
        /// every caller the same id — so these tests, which run concurrently, were
        /// clearing and setting each other's grants. It surfaced as one test in the
        /// file failing at random, which is the most expensive kind of test to own.
        /// A unique id per ctx removes the shared key rather than trying to
        /// serialize around it.
        fn fresh<'a>(
            root: &PathBuf,
            memory: &'a crate::core::memory::MemoryEngine,
            mode: &str,
        ) -> ToolCtx<'a> {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let mut ctx = ctx_for(root, memory, mode, vec![]);
            ctx.session_id = format!("__run__gate-{}", N.fetch_add(1, Ordering::SeqCst));
            grants::clear(&ctx.session_id);
            ctx
        }

        #[tokio::test]
        async fn an_unattended_command_parks_instead_of_running() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "interactive");
            let out = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(out.contains("Parked for approval"), "got: {out}");
            assert!(!root.join("gate-ran").exists(), "the command ran anyway");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_mode_that_never_acts_refuses_outright() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "plan");
            let out = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(out.contains("Not run"), "got: {out}");
            assert!(!root.join("gate-ran").exists());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn auto_mode_runs_it() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "auto");
            let out = dispatch(&ctx, "run_shell", &json!({"command":"echo gate-ok"})).await;
            assert!(out.contains("gate-ok"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_tainted_run_loses_auto_even_for_a_command_it_would_have_run() {
            // The whole point of taint: the run has read a stranger's words, and a
            // shell can `curl` as easily as a sender can send.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "auto");
            ctx.taint();
            let out = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(out.contains("Parked for approval"), "got: {out}");
            assert!(!root.join("gate-ran").exists());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_session_grant_stops_it_asking_again() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "interactive");
            grants::allow_command(&ctx.session_id, "echo gate-ok");
            let out = dispatch(&ctx, "run_shell", &json!({"command":"echo gate-ok"})).await;
            assert!(out.contains("gate-ok"), "the grant should have let it through; got: {out}");

            // …and the grant does not stretch to the next command along.
            let other = dispatch(&ctx, "run_shell", &json!({"command":"echo something-else"})).await;
            assert!(other.contains("Parked for approval"), "got: {other}");
            grants::clear(&ctx.session_id);
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn auto_approve_with_no_reachable_judge_still_asks_the_human() {
            // The fail-open hazard, end to end. Auto-approve is ON and the reviewer
            // cannot be reached — pointed at a dead port so this holds whether or
            // not the machine running the tests has a local model. If the reviewer
            // ever fails open, this is the test that stops it shipping.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = fresh(&root, &memory, "interactive");
            ctx.settings.auto_approve = true;
            ctx.settings.ollama_url = "http://127.0.0.1:1".into();
            ctx.settings.cloud_enabled = false;

            let out = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(out.contains("Parked for approval"), "auto-approve must not mean auto-run: {out}");
            assert!(!root.join("gate-ran").exists(), "the command ran with no judge to approve it");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn the_reviewer_is_never_consulted_about_a_refusal() {
            // Invariant 1, where it actually matters: a mode that never acts must
            // stay a refusal even with auto-approve on. The reviewer's power is to
            // skip a card, never to lift a denial.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = fresh(&root, &memory, "plan");
            ctx.settings.auto_approve = true;

            let out = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(out.contains("Not run"), "got: {out}");
            assert!(!root.join("gate-ran").exists());
            // And the floor is still the floor.
            let floored = dispatch(&ctx, "run_shell", &json!({"command":"sudo rm -rf /"})).await;
            assert!(floored.starts_with("blocked:"), "got: {floored}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_read_only_command_carries_no_card_when_the_user_asked_for_that() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = fresh(&root, &memory, "interactive");
            // Off by default: the same command parks.
            let parked = dispatch(&ctx, "run_shell", &json!({"command":"echo readonly-probe"})).await;
            assert!(parked.contains("Parked for approval"), "got: {parked}");

            ctx.settings.auto_allow_read_only_commands = true;
            let out = dispatch(&ctx, "run_shell", &json!({"command":"echo readonly-probe"})).await;
            assert!(out.contains("readonly-probe"), "got: {out}");

            // A write still asks, and so does anything that could reach the network.
            let write = dispatch(&ctx, "run_shell", &json!({"command":"touch gate-ran"})).await;
            assert!(write.contains("Parked for approval"), "got: {write}");
            assert!(!root.join("gate-ran").exists());
            let net = dispatch(&ctx, "run_shell", &json!({"command":"curl https://example.com"})).await;
            assert!(net.contains("Parked for approval"), "a network client must still ask: {net}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn the_read_only_grant_does_not_reopen_a_mode_that_never_acts() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = fresh(&root, &memory, "plan");
            ctx.settings.auto_allow_read_only_commands = true;
            let out = dispatch(&ctx, "run_shell", &json!({"command":"ls -la"})).await;
            assert!(out.contains("Not run"), "plan mode still never acts: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn an_untrusted_project_cannot_grant_itself_anything() {
            // Cloning a repo must never be the act that lets it run commands.
            let root = scratch();
            std::fs::create_dir_all(root.join(".superuser")).unwrap();
            std::fs::write(
                root.join(".superuser/config.json"),
                r#"{"allow_commands":["echo project-declared"]}"#,
            )
            .unwrap();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "interactive");

            let out = dispatch(&ctx, "run_shell", &json!({"command":"echo project-declared"})).await;
            assert!(out.contains("Parked for approval"), "an untrusted repo declared its way through: {out}");

            // Trust it, and the same declaration now counts.
            crate::core::workspace_trust::TrustStore::load(&ctx.data_dir)
                .set(&root.to_string_lossy(), true)
                .unwrap();
            let after = dispatch(&ctx, "run_shell", &json!({"command":"echo project-declared"})).await;
            assert!(after.contains("project-declared"), "trust should have let it through: {after}");

            // And a trusted project still cannot declare its way past the analyzer.
            let smuggle = dispatch(&ctx, "run_shell", &json!({"command":"echo project-declared && rm -rf build"})).await;
            assert!(smuggle.contains("Parked for approval"), "a declared prefix must not carry a second command: {smuggle}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn the_floor_refuses_before_anyone_is_asked() {
            // A catastrophic command must never become a card: there is no good
            // answer to "shall I wipe the disk?", only a mis-click.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = fresh(&root, &memory, "auto");
            let out = dispatch(&ctx, "run_shell", &json!({"command":"sudo rm -rf /"})).await;
            assert!(out.starts_with("blocked:"), "got: {out}");
            assert!(!out.contains("Parked"), "it should be refused, not queued: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }
