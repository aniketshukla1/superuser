//! end to end against a real server (cargo test -- --ignored)

use super::*;

    #[tokio::test]
    #[ignore = "requires Ollama with gemma4:12b and qwen2.5:7b-instruct pulled"]
    pub(super) async fn e2e_the_profile_reads_a_real_window_and_a_real_capability_list() {
        let s = Settings::default();
        let http = crate::core::egress::client("model");
        let p = resolve(&s, "gemma4:12b", &http, None).await;
        assert_eq!(p.source, ProfileSource::Discovered);
        // The number this phase exists for: the shipped `num_ctx` is 16384 and the
        // model holds sixteen times that. Knowing it is what makes the setting a
        // choice rather than a guess.
        assert_eq!(p.context_window, Some(262_144));
        assert!(p.vision, "gemma4 reads images; its name says nothing about it");
        assert!(p.can_think());
        // And the allocation is still the user's, not the model's ceiling.
        assert_eq!(p.ollama_num_ctx(&s), s.num_ctx);

        let q = resolve(&s, "qwen2.5:7b-instruct", &http, None).await;
        assert_eq!(q.context_window, Some(32_768));
        assert!(!q.vision);
        assert!(!q.can_think());
    }

    #[tokio::test]
    #[ignore = "requires Ollama serving its OpenAI-compatible surface on :11434"]
    pub(super) async fn e2e_the_probe_measures_a_server_that_refuses_reasoning_effort() {
        // Measured against the live endpoint: attempt one carries `reasoning_effort`
        // and comes back 400 `"qwen2.5:7b-instruct" does not support thinking`;
        // attempt two, without it, returns two tool calls and a usage block. That
        // difference is the whole point of a handshake — the same 400, sent inside a
        // real turn, discards every tool call the turn had already run.
        let http = crate::core::egress::client("model");
        let m = probe_openai(
            &http,
            "http://127.0.0.1:11434/v1/chat/completions",
            None,
            "qwen2.5:7b-instruct",
        )
        .await
        .expect("probe should reach the endpoint");
        assert!(!m.accepts_effort, "the server refuses reasoning_effort");
        assert!(m.reports_usage);
        // Two calls in one message. The tag listing says only "tools" — it has no
        // word for parallel — so this is a fact only the handshake can establish.
        assert_eq!(m.tools, ToolSupport::Parallel);
    }

    #[test]
    pub(super) fn codex_answers_none_rather_than_answering_for_a_loop_it_does_not_serve() {
        // Picking Codex hands the whole turn to the CLI's own agent. Our loop's
        // questions have no meaning there, and a plausible answer invites a caller to
        // act on one.
        let p = declared("codex:gpt-5.6-sol").unwrap();
        assert_eq!(p.tools, ToolSupport::None);
        assert_eq!(p.thinking, None);
    }
