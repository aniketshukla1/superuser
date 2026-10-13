use super::*;

    /// A response whose body sends `first` and then nothing, forever — a provider
    /// that accepted the request and went quiet. No socket needed.
    pub(super) fn goes_quiet(first: &'static str) -> reqwest::Response {
        use futures_util::StreamExt;
        let body = futures_util::stream::iter(vec![Ok::<_, std::io::Error>(bytes_of(first))])
            .chain(futures_util::stream::pending());
        reqwest::Response::from(http::Response::new(reqwest::Body::wrap_stream(body)))
    }

    pub(super) fn bytes_of(s: &'static str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[tokio::test]
    pub(super) async fn a_stream_that_goes_quiet_is_abandoned_with_what_it_had_not_as_an_answer() {
        let resp = goes_quiet("data: {\"choices\":[{\"delta\":{\"content\":\"Half an\"}}]}\n");
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let began = std::time::Instant::now();
        let out = stream_chat_message(resp, true, &cancel, Some(std::time::Duration::from_millis(700)), |_| {}, |_| {})
            .await
            .unwrap();
        assert!(out.stalled.is_some(), "the silence was noticed");
        assert!(!out.stopped);
        assert!(began.elapsed() < std::time::Duration::from_secs(3), "and within the limit: {:?}", began.elapsed());
    }

    #[tokio::test]
    pub(super) async fn stop_lands_during_a_silence_too() {
        // A bare `next().await` could only notice Stop when a chunk arrived — so a
        // stalled stream could not be stopped either. It now wakes to check.
        let resp = goes_quiet("data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n");
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            c.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        let out = stream_chat_message(resp, true, &cancel, None, |_| {}, |_| {}).await.unwrap();
        assert!(out.stopped && out.stalled.is_none());
    }

    /// Replay a wire transcript line by line, as the socket delivers it.
    pub(super) fn replay(lines: &[&str], openai: bool) -> (String, serde_json::Value, bool) {
        let mut acc = StreamAccum::default();
        let mut shown = String::new();
        for l in lines {
            if let Some(t) = acc.push_line(l, openai).text {
                shown.push_str(&t);
            }
        }
        let done = acc.done;
        (shown, acc.finish(), done)
    }

    #[test]
    pub(super) fn ollama_stream_paints_text_and_keeps_the_final_message() {
        let (shown, msg, done) = replay(
            &[
                r#"{"message":{"role":"assistant","content":"Cre"},"done":false}"#,
                r#"{"message":{"role":"assistant","content":"ated"},"done":false}"#,
                r#"{"message":{"role":"assistant","content":" it."},"done":true}"#,
            ],
            false,
        );
        // Every fragment reached the user as it arrived — that's the point of streaming.
        assert_eq!(shown, "Created it.");
        assert_eq!(msg["content"], "Created it.");
        assert!(msg["tool_calls"].is_null(), "no tool_calls key when there were none");
        assert!(done);
    }

    #[test]
    pub(super) fn reasoning_is_kept_out_of_the_answer() {
        // Ollama reports a thinking model's working in `message.thinking`, separately
        // from `message.content`. They must stay separate: reasoning belongs in the
        // collapsed Thinking block, and letting it into `content` would paste the
        // model's private deliberation into the transcript as if it were the answer.
        let mut acc = StreamAccum::default();
        let d1 = acc.push_line(r#"{"message":{"role":"assistant","content":"","thinking":"Let me add "},"done":false}"#, false);
        let d2 = acc.push_line(r#"{"message":{"role":"assistant","content":"","thinking":"10 and 15.99."},"done":false}"#, false);
        let d3 = acc.push_line(r#"{"message":{"role":"assistant","content":"It is 25.99."},"done":true}"#, false);

        assert_eq!(d1.thinking.as_deref(), Some("Let me add "));
        assert_eq!(d1.text, None, "a thinking-only chunk must not paint into the answer");
        assert_eq!(d2.thinking.as_deref(), Some("10 and 15.99."));
        assert_eq!(d3.text.as_deref(), Some("It is 25.99."));

        assert_eq!(acc.thinking, "Let me add 10 and 15.99.");
        assert_eq!(acc.content, "It is 25.99.", "content must hold the answer only");
        assert!(acc.done);
    }

    /// Token usage, from both wire formats.
    ///
    /// This is the whole reason the app never had a token count: OpenAI puts usage in a
    /// trailing chunk whose `choices` array is EMPTY, and sends it only when the request
    /// asked. Ollama puts its counts on the final `done` object under entirely different
    /// names. Neither looks like the other, and a parser that reads only `choices[0].delta`
    /// sees neither.
    #[test]
    pub(super) fn a_streamed_turn_reports_what_it_cost() {
        // OpenAI: the usage chunk carries no choices at all.
        let mut a = StreamAccum::default();
        a.push_line(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#, true);
        a.push_line(r#"data: {"choices":[],"usage":{"prompt_tokens":120,"completion_tokens":34}}"#, true);
        a.push_line("data: [DONE]", true);
        let u = a.usage.expect("openai usage went unread");
        assert_eq!((u.input, u.output, u.total()), (120, 34, 154));
        assert_eq!(a.content, "hi", "the usage chunk must not disturb the answer");

        // Ollama: different names, on the final object.
        let mut b = StreamAccum::default();
        b.push_line(r#"{"message":{"content":"hi"}}"#, false);
        b.push_line(r#"{"done":true,"prompt_eval_count":88,"eval_count":12}"#, false);
        let u = b.usage.expect("ollama usage went unread");
        assert_eq!((u.input, u.output), (88, 12));

        // A provider that says nothing stays None. Zero is a claim; silence is not.
        let mut c = StreamAccum::default();
        c.push_line(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#, true);
        c.push_line("data: [DONE]", true);
        assert!(c.usage.is_none(), "silence must not read as a free turn");

        // Split across chunks, where the later one OMITS the field the earlier one carried.
        // Assigning rather than accumulating loses the input count here — and an earlier
        // version of this test reused the same prompt_tokens in both chunks, so assignment
        // and accumulation produced the same answer and the mutation went uncaught.
        let mut d = StreamAccum::default();
        d.push_line(r#"data: {"choices":[],"usage":{"prompt_tokens":500}}"#, true);
        d.push_line(r#"data: {"choices":[],"usage":{"completion_tokens":7}}"#, true);
        let u = d.usage.unwrap();
        assert_eq!((u.input, u.output), (500, 7), "a later chunk erased an earlier count");
    }

    /// Empty usage is not stored: "the provider said nothing" and "the turn was free" must
    /// not look alike once it reaches disk.
    #[test]
    pub(super) fn only_reads_may_share_a_moment() {
        use crate::core::agent::{RiskClass, ToolSpec};
        let spec = |name: &str, risk| ToolSpec {
            name: name.to_string(),
            description: String::new(),
            capability: String::new(),
            risk,
            schema: serde_json::json!({}),
        };
        let specs = vec![
            spec("read_file", RiskClass::Read),
            spec("write_file", RiskClass::WriteLocal),
            spec("run_shell", RiskClass::Exec),
            spec("web_fetch", RiskClass::External),
        ];
        let mcp = McpBundle::default();

        // A read returns content and changes nothing, so two overlapping cannot
        // produce a different result than running them in order.
        assert!(parallel_safe("read_file", &specs, &mcp));

        // Everything else stays in the model's emitted order. `write_file` and
        // `run_shell` can ask for approval, and two prompts in flight at once is not
        // a thing this UI can show; `web_fetch` leaves the machine.
        assert!(!parallel_safe("write_file", &specs, &mcp));
        assert!(!parallel_safe("run_shell", &specs, &mcp));
        assert!(!parallel_safe("web_fetch", &specs, &mcp));

        // A tool nobody declared is not assumed safe.
        assert!(!parallel_safe("something_new", &specs, &mcp));
    }
