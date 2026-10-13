use super::*;

    /// Drive a request through the real route table and read the answer back.
    ///
    /// Through the router rather than calling the handler directly, because a
    /// handler called directly still passes when its route was never registered or
    /// its path pattern matches nothing — which is the failure that actually
    /// happened here, twice.
    pub(super) async fn hit(method: &str, path: &str, body: Value) -> (axum::http::StatusCode, Value) {
        use tower::ServiceExt;
        let payload = if body.is_null() {
            axum::body::Body::empty()
        } else {
            axum::body::Body::from(body.to_string())
        };
        let req = axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(payload)
            .unwrap();
        let res = provider_routes::<()>().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 256 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    /// `api.ts` calls four provider routes. Three of them were never registered, so
    /// save, verify and remove answered 404 and the setup form could not store
    /// anything; the delete path was then written in axum 0.8's `{name}` syntax
    /// against axum 0.7, which compiles and matches nothing.
    ///
    /// An unknown provider is used on purpose: the handler rejects it before it
    /// touches the credential store, so this proves the route is reachable without
    /// writing to the real secrets file — which is the user's own, not a test one.
    #[tokio::test]
    pub(super) async fn the_provider_routes_are_the_ones_the_client_calls() {
        for (method, path) in [("POST", "/v1/providers"), ("DELETE", "/v1/providers/nope")] {
            let (status, body) = hit(method, path, json!({ "name": "nope" })).await;
            assert_eq!(status, 200, "{method} {path} never reached a handler");
            assert_eq!(body["ok"], json!(false));
            assert!(
                body["error"].as_str().unwrap_or("").contains("unknown provider"),
                "{method} {path} answered {body}"
            );
        }
    }

    /// `ProviderField` in api.ts reads `f.key`. This route emitted `name`, so every
    /// field arrived as undefined and the form rendered rows that could never be
    /// filled in or saved. Read-only: it asserts the shape, not anything stored.
    #[tokio::test]
    pub(super) async fn the_setup_form_gets_fields_it_can_actually_render() {
        let (status, body) = hit("GET", "/v1/providers", Value::Null).await;
        assert_eq!(status, 200);
        let all = body.as_array().expect("a bare array, as the picker reads it");
        let field_keys = |id: &str| -> Vec<String> {
            all.iter()
                .find(|p| p["name"] == id)
                .unwrap_or_else(|| panic!("no provider {id}"))["fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f["key"].as_str().unwrap_or_default().to_string())
                .collect()
        };
        let openai = field_keys("openai");
        assert!(openai.contains(&"api_key".to_string()), "got {openai:?}");
        // Offered on every cloud provider, not just `custom`: pointing an existing
        // one at a gateway keeps that provider's curated model ids.
        assert!(openai.contains(&"base_url".to_string()), "got {openai:?}");
        // `custom` also needs a model list, since we cannot curate one for it.
        assert!(field_keys("custom").contains(&"models".to_string()));
    }

    /// A refused endpoint must be refused *before* anything is written. A save that
    /// stored the URL and then reported an error would leave the provider pointed
    /// somewhere that fails on the next turn, with the UI saying it had failed.
    #[tokio::test]
    pub(super) async fn a_bad_endpoint_is_refused_before_anything_is_written() {
        let before = crate::core::providers::base_url("openai");
        let (status, body) = hit(
            "POST",
            "/v1/providers",
            json!({ "name": "openai", "fields": { "base_url": "api.openai.com/v1" } }),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(body["ok"], json!(false), "a schemeless URL must not be accepted");
        assert!(body["error"].as_str().unwrap_or("").contains("http://"), "{body}");
        assert_eq!(
            crate::core::providers::base_url("openai"),
            before,
            "a refused save wrote to the store anyway"
        );
    }

    #[test]
    pub(super) fn a_steer_is_delivered_once_and_only_to_its_own_session() {
        let a = format!("steer-a-{}", std::process::id());
        let b = format!("steer-b-{}", std::process::id());
        queue_steer(&a, "use staging, not prod");
        queue_steer(&a, "and skip the tests");
        queue_steer(&b, "different session");

        // Draining hands the run everything waiting, in the order it was typed.
        assert_eq!(take_steers(&a), vec!["use staging, not prod", "and skip the tests"]);
        // …and leaves nothing behind, so the next step doesn't re-apply it.
        assert!(take_steers(&a).is_empty());
        // A steer meant for one conversation never reaches another.
        assert_eq!(take_steers(&b), vec!["different session"]);
    }

    #[test]
    pub(super) fn a_steer_that_missed_its_turn_is_dropped_rather_than_carried_forward() {
        // Applying an instruction to the *next*, unrelated turn would be worse than
        // losing it — and it is still in the transcript either way.
        let id = format!("steer-late-{}", std::process::id());
        queue_steer(&id, "too late");
        clear_steers(&id);
        assert!(take_steers(&id).is_empty());
    }

    #[test]
    pub(super) fn holding_down_enter_cannot_queue_an_unbounded_pile() {
        let id = format!("steer-flood-{}", std::process::id());
        for i in 0..50 {
            queue_steer(&id, &format!("msg {i}"));
        }
        let got = take_steers(&id);
        assert_eq!(got.len(), 8, "bounded");
        assert_eq!(got[0], "msg 0", "the earliest are kept — later ones are the ones lost");
    }

    #[test]
    pub(super) fn compact_keeps_a_sane_number_of_turns() {
        assert_eq!(compact_keep_turns(20), 5);
        assert_eq!(compact_keep_turns(40), 10);
        // A tiny budget must still leave a coherent exchange behind.
        assert_eq!(compact_keep_turns(4), 2);
        assert_eq!(compact_keep_turns(2), 2);
    }

    #[test]
    pub(super) fn compact_split_lands_on_a_turn_boundary() {
        use crate::core::sessions::Message;
        let mut msgs = Vec::new();
        for i in 0..6 {
            msgs.push(Message::new("user", format!("q{i}")));
            msgs.push(Message::new("assistant", format!("a{i}")));
        }
        // Keep 2 turns of 6 → split at the start of turn 5 (index 8).
        let split = compact_split(&msgs, 0, 2).unwrap();
        assert_eq!(split, 8);
        assert_eq!(msgs[split].role, "user", "never splits mid-turn");
    }

    #[test]
    pub(super) fn compact_declines_when_there_is_nothing_older_to_fold() {
        use crate::core::sessions::Message;
        let msgs = vec![
            Message::new("user", "q0"),
            Message::new("assistant", "a0"),
            Message::new("user", "q1"),
            Message::new("assistant", "a1"),
        ];
        assert_eq!(compact_split(&msgs, 0, 2), None, "exactly the kept turns");
        assert_eq!(compact_split(&[], 0, 2), None);
        assert!(compact_split(&msgs, 0, 1).is_some());
    }

    #[test]
    pub(super) fn compacting_twice_only_considers_what_is_not_already_summarized() {
        use crate::core::sessions::Message;
        let mut msgs = Vec::new();
        for i in 0..8 {
            msgs.push(Message::new("user", format!("q{i}")));
            msgs.push(Message::new("assistant", format!("a{i}")));
        }
        // Already compacted through turn 4; 4 turns remain, keeping 2 leaves 2 to fold.
        let split = compact_split(&msgs, 8, 2).unwrap();
        assert_eq!(split, 12);
        // With only the kept turns left after the previous compaction, it declines
        // rather than re-summarizing a summary on every invocation.
        assert_eq!(compact_split(&msgs, 12, 2), None);
    }
