use super::*;

    /// A required field the server never sends is the other half of a missing route,
    /// and it is quieter: no 404, no failed request, just a control that cannot
    /// render because the thing it renders is `undefined`.
    ///
    /// The connector cards were the case that proved it. `allowed_users` and `tools`
    /// are non-optional in the client's `Connector`, nothing sent them, and every
    /// panel guarded on `c.tools?.length` — so the allowlist chips and the tool
    /// toggles were invisible in a UI that looked complete, and the six routes
    /// behind them had never once been called.
    #[test]
    pub(super) fn every_required_field_of_the_connector_contract_is_sent() {
        let Ok(src) = crate::test_support::gui_source("api.ts") else {
            return; // packaged builds ship no client source
        };
        let Some(start) = src.find("export interface Connector {") else {
            panic!("the Connector interface moved — this test is reading the wrong thing");
        };
        let body = &src[start..][..src[start..].find("\n}").expect("unterminated interface")];

        // `foo: T` is required; `foo?: T` is not.
        let required: Vec<&str> = body
            .lines()
            .skip(1)
            .filter_map(|l| {
                let l = l.trim();
                if l.starts_with("//") {
                    return None;
                }
                let (name, rest) = l.split_once(':')?;
                (!name.ends_with('?') && !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                    .then_some(())?;
                let _ = rest;
                Some(name)
            })
            .collect();
        assert!(required.len() > 10, "parsed {} fields — the reader has drifted", required.len());

        let server = crate::test_support::module_source("core/api_server");
        let handler = {
            let at = server.find("async fn connectors_list").expect("handler renamed");
            let tail = &server[at..];
            &tail[..tail.find("\n/// Ids allowed to reach").unwrap_or(tail.len())]
        };
        let missing: Vec<&str> = required.into_iter().filter(|f| !handler.contains(&format!("\"{f}\""))).collect();
        assert!(
            missing.is_empty(),
            "the client's Connector type requires these and connectors_list sends none of them:\n  {}\n\n\
             The card reads `undefined` and renders nothing. Send the field — an empty list or a \
             plain `false` is an answer; absence is not.",
            missing.join("\n  ")
        );
    }

    #[tokio::test]
    pub(super) async fn a_connectors_tools_route_lists_what_assistants_use_there() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        async fn json_of(res: axum::response::Response) -> Value {
            serde_json::from_slice(&axum::body::to_bytes(res.into_body(), 1 << 16).await.unwrap()).unwrap()
        }
        // The real table, read above the tests: a route only this test registered would
        // pass while the app still 404'd.
        let all = crate::test_support::module_source("core/api_server");
        let code = all.split("#[cfg(test)]").next().unwrap();
        assert!(
            code.contains(r#".route("/v1/connectors/:name/tools", get(connector_tools_api))"#),
            "the route table no longer sends this path to connector_tools_api"
        );
        let app = Router::new().route("/v1/connectors/:name/tools", get(connector_tools_api));

        let res = app.clone().oneshot(Request::get("/v1/connectors/prometheus/tools").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let out = json_of(res).await;
        assert_eq!(out["ok"], true);
        let tools = out["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["metrics_find", "metrics_query", "metrics_range", "metrics_compare", "metrics_alerts", "perf_report"]);
        for t in tools {
            // The client's `ConnectorTool`, field for field.
            for field in ["name", "label", "kind", "description", "enabled", "requires_approval"] {
                assert!(t.get(field).is_some(), "{} has no `{field}`", t["name"]);
            }
            assert_ne!(t["label"], t["name"], "a label, not the identifier");
            // Read-only, all of them: nothing here writes to Prometheus or waits for a yes.
            assert_eq!(t["kind"], "read");
            assert_eq!(t["requires_approval"], false);
            assert_eq!(t["enabled"], true);
        }

        let out = json_of(app.oneshot(Request::get("/v1/connectors/carrier-pigeon/tools").body(Body::empty()).unwrap()).await.unwrap()).await;
        assert_eq!(out["ok"], false);
        assert_eq!(out["tools"], json!([]));
    }

    #[test]
    pub(super) fn a_connector_lists_tools_exactly_when_it_has_a_client() {
        // "No tools for this connector yet" is still rendered for an empty list, so an
        // empty list has to mean it: a connector the catalogue names and nothing reaches.
        for c in crate::integrations::descriptors::CONNECTORS {
            let tools = connector_tools(c.id);
            assert_eq!(!tools.is_empty(), c.implemented, "{}: implemented is {}, and it lists {} tool(s)", c.id, c.implemented, tools.len());
            for t in &tools {
                let name = t["name"].as_str().unwrap();
                assert!(tool_copy(name).is_some(), "{} lists `{name}` without a label of its own", c.id);
            }
        }
        let names = |id: &str| connector_tools(id).iter().map(|t| t["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names("slack"), ["connector_search", "connector_sync", "connector_changes", "send_message"]);
        assert_eq!(names("telegram"), ["send_message"], "reached by message; nothing is synced from it");

        // Only the tool that posts waits for approval.
        let slack = connector_tools("slack");
        let tool = |n: &str| slack.iter().find(|t| t["name"] == n).unwrap();
        assert_eq!(tool("send_message")["kind"], "write");
        assert_eq!(tool("send_message")["requires_approval"], true);
        assert_eq!(tool("connector_sync")["kind"], "read", "it reaches the app, but only reads it");
        assert_eq!(tool("connector_sync")["requires_approval"], false);
    }

    #[test]
    pub(super) fn the_surfaces_payload_uses_the_keys_the_client_reads() {
        // The client's SurfaceVisibility is {cowork, chat, code}. Sending anything
        // else is not a 404 — it is worse, because the object is truthy and the GUI
        // adopts it, replacing correct defaults with undefined.
        let mut s = crate::core::settings::Settings::default();
        s.surface_chat = true;
        let v = surfaces_payload(&s);
        assert_eq!(v["cowork"], true, "cowork is always visible");
        assert_eq!(v["chat"], true);
        assert_eq!(v["code"], false);
        // And nothing from the concept that used to be sent under this name.
        for stale in ["automations", "inbox", "connectors", "personas", "skills"] {
            assert!(v.get(stale).is_none(), "`{stale}` is a different thing entirely");
        }
    }

    #[test]
    pub(super) fn a_shell_approval_arrives_with_the_command_its_card_renders() {
        // `ApprovalCard` reads `args.command` to show what would run, and its
        // "Always allow this command" button is keyed off `name === "run_shell"`.
        // A frame missing either leaves the user approving an unnamed action.
        let out = gui_frame(&json!({
            "type": "permission",
            "name": "run_shell",
            "arguments": { "command": "cargo test --release" },
            "reason": "Commands are confirmed before they run.",
            "category": "shell",
        }))
        .unwrap();
        assert_eq!(out["type"], "permission_required");
        assert_eq!(out["data"]["name"], "run_shell");
        assert_eq!(out["data"]["arguments"]["command"], "cargo test --release");
        assert_eq!(out["data"]["category"], "shell");
    }

    #[test]
    pub(super) fn a_question_arrives_with_everything_its_card_needs() {
        // The card renders each option's label AND the line under it, and steps
        // through several questions. Dropping any of it leaves the user looking at a
        // question they cannot answer well, on a turn that is blocked.
        let out = gui_frame(&json!({
            "type": "question",
            "questions": [{
                "question": "Where is this initiative right now?",
                "multi": false,
                "options": [
                    { "label": "Still pre-presentation", "description": "The deck hasn't been shown yet." },
                    { "label": "Presented, now piloting", "description": "Phase 1 is executing." }
                ]
            }]
        }))
        .unwrap();
        assert_eq!(out["type"], "question_requested");
        let q = &out["data"]["questions"][0];
        assert_eq!(q["question"], "Where is this initiative right now?");
        assert_eq!(q["options"][1]["label"], "Presented, now piloting");
        assert_eq!(q["options"][1]["description"], "Phase 1 is executing.");
        assert_eq!(q["multi"], false);
    }
