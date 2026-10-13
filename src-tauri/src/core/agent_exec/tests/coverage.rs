use super::*;
        use crate::core::toolchain::{self, Presence};

        #[tokio::test]
        async fn an_unknown_scanner_is_refused_and_names_what_is_known() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            let out = dispatch(&ctx, "check_tool", &json!({"name": "curl; rm -rf /"})).await;
            assert!(out.starts_with("error:"), "got: {out}");
            assert!(out.contains("gitleaks"), "it should say what IS known: {out}");
            // Nothing unknown reaches the ledger either — a line for a tool we
            // cannot describe would be a coverage claim about nothing.
            assert!(ctx.coverage.lock().unwrap().is_empty());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn checking_a_scanner_always_leaves_a_line_in_the_note() {
            // The invariant, whichever way this machine answers: once the agent has
            // asked about a scanner, that check exists in the record. It cannot be
            // discovered to be missing and then quietly not mentioned.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            let installed = matches!(toolchain::probe(toolchain::find("gitleaks").unwrap()), Presence::Present { .. });

            let out = dispatch(&ctx, "check_tool", &json!({"name": "gitleaks"})).await;
            let note = dispatch(&ctx, "coverage_note", &json!({})).await;
            let check = toolchain::find("gitleaks").unwrap().check;
            assert!(note.contains(check), "the check must appear in the note: {note}");
            if installed {
                assert!(out.contains("is installed"), "got: {out}");
                assert!(note.contains("ran under"), "{note}");
            } else {
                assert!(out.contains("NOT RUN"), "got: {out}");
                assert!(note.contains("NOT RUN"), "{note}");
                assert!(note.contains("only that they were not looked at"), "{note}");
            }
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_skipped_check_can_be_amended_to_degraded_but_never_erased() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            let check = toolchain::find("gitleaks").unwrap().check;

            dispatch(&ctx, "check_tool", &json!({"name": "gitleaks"})).await;
            dispatch(&ctx, "record_coverage", &json!({
                "check": check, "status": "degraded", "detail": "swept the history by hand with git log -p"
            })).await;

            let note = dispatch(&ctx, "coverage_note", &json!({})).await;
            assert_eq!(note.matches(check).count(), 1, "one check, one line:\n{note}");
            assert!(note.contains("degraded: swept the history by hand"), "{note}");
            // Amending is the only edit there is — there is no path that removes a line.
            assert_eq!(ctx.coverage.lock().unwrap().len(), 1);
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn a_coverage_line_with_no_reason_is_refused() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            let out = dispatch(&ctx, "record_coverage", &json!({"check": "Secret sweep", "status": "skipped", "detail": "  "})).await;
            assert!(out.starts_with("error:"), "got: {out}");
            let bad = dispatch(&ctx, "record_coverage", &json!({"check": "X", "status": "probably", "detail": "y"})).await;
            assert!(bad.starts_with("error:"), "got: {bad}");
            assert!(ctx.coverage.lock().unwrap().is_empty());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn an_unattended_run_degrades_rather_than_queueing_an_install() {
            // A scheduled review that stops to wait for a Homebrew approval is a
            // review that silently produced nothing.
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]); // ctx_for is unattended
            assert!(ctx.unattended);
            let t = toolchain::find("trivy").unwrap();
            let out = dispatch(&ctx, "request_tool", &json!({"name": "trivy", "reason": "scan the Terraform"})).await;
            match toolchain::probe(t) {
                Presence::Present { .. } => assert!(out.contains("already installed"), "got: {out}"),
                Presence::Missing => {
                    assert!(out.contains("Nobody is attending"), "got: {out}");
                    assert!(!out.contains("Parked"), "an install must not be queued: {out}");
                    let note = dispatch(&ctx, "coverage_note", &json!({})).await;
                    assert!(note.contains("NOT RUN"), "{note}");
                }
            }
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn an_empty_ledger_does_not_pretend_to_be_coverage() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ctx_for(&root, &memory, "interactive", vec![]);
            let note = dispatch(&ctx, "coverage_note", &json!({})).await;
            assert!(note.contains("no checks were registered"), "{note}");
            let _ = std::fs::remove_dir_all(&root);
        }
