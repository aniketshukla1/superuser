use super::*;

        pub(super) fn ready<'a>(root: &PathBuf, memory: &'a crate::core::memory::MemoryEngine, persona: &str, mode: &str, unattended: bool) -> ToolCtx<'a> {
            let mut ctx = ctx_for(root, memory, mode, vec![]);
            ctx.settings.browser_enabled = true;
            ctx.persona = persona.into();
            ctx.session_id = format!("__run__{persona}");
            ctx.unattended = unattended;
            ctx
        }

        #[tokio::test]
        pub(super) async fn switched_off_it_says_where_to_turn_it_on_and_creates_nothing() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, "bt-off", "auto", false);
            ctx.settings.browser_enabled = false;
            let out = dispatch(&ctx, "browser_navigate", &json!({"url": "https://example.com"})).await;
            assert!(out.starts_with("error:") && out.contains("Settings"), "got: {out}");
            assert!(computer::existing("bt-off").is_none(), "a switched-off browser was set up anyway");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn bug_reproduction_opens_staging_and_nothing_else() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, crate::core::repro::ASSISTANT, "auto", false);
            let out = dispatch(&ctx, "browser_navigate", &json!({"url": "https://staging.example.com"})).await;
            assert!(out.starts_with("error:") && out.contains("no staging address"), "without one set, nothing opens: {out}");
            ctx.settings.repro_staging = "https://staging.example.com".into();
            for url in ["https://app.example.com/admin", "app.example.com", "https://staging.example.com.evil.io/"] {
                let out = dispatch(&ctx, "browser_navigate", &json!({"url": url})).await;
                assert!(out.starts_with("error:") && out.contains("never production"), "{url}: {out}");
            }
            ctx.settings.repro_staging = "http://localhost:3000".into();
            let out = dispatch(&ctx, "browser_navigate", &json!({"url": "http://localhost:8080/admin"})).await;
            assert!(out.contains("isn't staging"), "another service on the same machine: {out}");
            assert!(computer::existing(crate::core::repro::ASSISTANT).is_none(), "a refused address still set up a browser");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn a_mode_that_never_acts_and_a_run_nobody_watches_do_not_click() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let click = json!({"ref": "e1", "snapshot_id": 1});
            let out = dispatch(&ready(&root, &memory, "bt-plan", "plan", false), "browser_click", &click).await;
            assert!(out.starts_with("Not done") && out.contains("plan"), "got: {out}");
            let out = dispatch(&ready(&root, &memory, "bt-unwatched", "auto", true), "browser_type", &json!({"ref": "e1", "snapshot_id": 1, "text": "hi"})).await;
            assert!(out.starts_with("Not done") && out.contains("nobody is watching"), "got: {out}");
            // Refused before the browser was even looked up.
            assert!(computer::existing("bt-plan").is_none() && computer::existing("bt-unwatched").is_none());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn a_click_is_asked_about_and_a_deny_means_nothing_happens() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, "bt-ask-deny", "interactive", false);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            ctx.events = Some(tx);
            let sid = ctx.session_id.clone();
            let args = json!({"ref": "e1", "snapshot_id": 1});
            let clicked = dispatch(&ctx, "browser_click", &args);
            let user = async {
                let ev = rx.recv().await.expect("nobody was asked");
                assert_eq!((ev["type"].as_str(), ev["category"].as_str()), (Some("permission"), Some("browser")));
                // Words a person can judge, not the model's handles.
                assert!(ev["arguments"].get("ref").is_none() && ev["arguments"].get("snapshot_id").is_none(), "{ev}");
                while !crate::core::ask::is_waiting(&sid) {
                    tokio::task::yield_now().await;
                }
                crate::core::ask::answer(&sid, "deny");
            };
            let (out, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async { tokio::join!(clicked, user) }).await.unwrap();
            assert!(out.starts_with("Denied by the user"), "got: {out}");
            assert!(computer::existing("bt-ask-deny").is_none(), "a denied click still set up a browser");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        pub(super) fn a_site_is_its_host_and_the_port_it_names() {
            assert_eq!(site_of("https://shop.example/cart"), "shop.example");
            assert_eq!(site_of("https://shop.example:443/cart"), "shop.example", "a scheme's own port is the same site");
            assert_eq!(site_of("http://127.0.0.1:3000/"), "127.0.0.1:3000");
            assert_ne!(site_of("http://127.0.0.1:3000/"), site_of("http://127.0.0.1:8080/"), "two programs, two sites");
            assert_eq!(site_of("http://[::1]:5173/"), "[::1]:5173");
            assert_eq!(site_of("https://Shop.Example.:8443/x"), "shop.example:8443");
            assert_eq!(site_of("about:blank"), "");
        }

        #[tokio::test]
        pub(super) async fn always_allow_covers_that_site_for_the_conversation_and_no_other() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ready(&root, &memory, "bt-ask-grant", "interactive", false);
            // No browser and no page: the site is "" — granted as such, and a grant for a
            // real host must not cover it.
            crate::core::cmdgate::grants::allow_tool(&ctx.session_id, "browser@shop.example");
            assert!(browser_approval(&ctx, "browser_click", &json!({"ref": "e1", "snapshot_id": 1})).await.is_some());
            crate::core::cmdgate::grants::allow_tool(&ctx.session_id, "browser@");
            assert_eq!(browser_approval(&ctx, "browser_click", &json!({"ref": "e1", "snapshot_id": 1})).await, None);
            crate::core::cmdgate::grants::clear(&ctx.session_id);
            // `auto` on a run that has read nothing goes ahead; one that has read a page asks.
            let auto = ready(&root, &memory, "bt-ask-auto", "auto", false);
            assert_eq!(browser_approval(&auto, "browser_key", &json!({"key": "Enter"})).await, None);
            auto.taint();
            let asked = browser_approval(&auto, "browser_key", &json!({"key": "Enter"})).await.unwrap();
            assert!(asked.contains("nothing is attached"), "a tainted run went ahead: {asked}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn a_ref_without_its_snapshot_is_sent_back_before_anything_starts() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ready(&root, &memory, "bt-noid", "auto", false);
            let out = dispatch(&ctx, "browser_click", &json!({"ref": "e1"})).await;
            assert!(out.starts_with("error:") && out.contains("snapshot_id"), "got: {out}");
            assert!(!computer::existing("bt-noid").unwrap().is_running().await, "Chrome was started for a call that can't run");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn help_asked_for_in_an_unwatched_run_shows_on_the_browser_and_parks_nothing() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ready(&root, &memory, "bt-help-unwatched", "auto", true);
            let out = dispatch(&ctx, "browser_request_help", &json!({"reason": "Sign in to example.com"})).await;
            assert!(out.contains("Nobody is watching"), "got: {out}");
            assert!(computer::existing("bt-help-unwatched").unwrap().control().needs_you());
            // Approving an Inbox item replays it, and a replayed request for hands has
            // nobody's hands to wait for.
            assert!(crate::core::inbox::Inbox::load(&root).pending().is_empty());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn help_with_nobody_attached_says_so_instead_of_waiting() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let ctx = ready(&root, &memory, "bt-help-alone", "auto", false);
            let out = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                dispatch(&ctx, "browser_request_help", &json!({"reason": "Sign in"})),
            )
            .await
            .expect("it waited for a person who can't be reached");
            assert!(out.contains("nothing is attached"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[tokio::test]
        pub(super) async fn the_wait_for_help_ends_when_the_user_hands_the_browser_back() {
            let root = scratch();
            let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
            let mut ctx = ready(&root, &memory, "bt-help-back", "auto", false);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            ctx.events = Some(tx);
            let args = json!({"reason": "Sign in to example.com"});
            let asked = dispatch(&ctx, "browser_request_help", &args);
            let user = async {
                let ev = rx.recv().await.expect("the ask never reached the user");
                assert_eq!((ev["type"].as_str(), ev["reason"].as_str()), (Some("computer_help"), Some("Sign in to example.com")));
                let c = computer::existing("bt-help-back").unwrap();
                assert!(c.control().needs_you());
                c.take_control();
                c.release_control();
            };
            let (out, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async { tokio::join!(asked, user) })
                .await
                .expect("handing the browser back didn't end the wait");
            assert!(out.contains("handed your browser back") && out.contains("new snapshot"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }
