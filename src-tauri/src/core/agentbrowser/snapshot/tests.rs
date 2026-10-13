use super::*;
    use serde_json::json;

    fn node(id: &str, role: &str, name: &str, children: &[&str], extra: Value) -> Value {
        let mut n = json!({
            "nodeId": id,
            "ignored": false,
            "role": {"type": "role", "value": role},
            "name": {"type": "computedString", "value": name},
            "childIds": children,
            "backendDOMNodeId": id.parse::<i64>().unwrap_or(0) + 100,
        });
        if let Value::Object(m) = extra {
            for (k, v) in m {
                n[k] = v;
            }
        }
        n
    }

    fn page() -> Vec<Value> {
        vec![
            json!({"nodeId": "1", "ignored": false, "role": {"value": "RootWebArea"}, "name": {"value": "Sign in"}, "childIds": ["2", "6"], "backendDOMNodeId": 1}),
            // An ignored wrapper with actionable children, listed out of order on purpose.
            json!({"nodeId": "2", "ignored": true, "role": {"value": "generic"}, "childIds": ["4", "3"], "parentId": "1"}),
            node("4", "textbox", "Password", &[], json!({"parentId": "2"})),
            node("3", "textbox", "Email", &[], json!({"parentId": "2", "value": {"type": "string", "value": "a@b.com"}})),
            node("6", "group", "Options", &["7", "8", "9"], json!({"parentId": "1"})),
            node("7", "checkbox", "Remember me", &[], json!({"parentId": "6"})),
            node("8", "checkbox", "Select all", &[], json!({"parentId": "6", "properties": [{"name": "checked", "value": {"type": "tristate", "value": "mixed"}}]})),
            node("9", "button", "Sign in", &[], json!({"parentId": "6", "properties": [{"name": "disabled", "value": {"type": "boolean", "value": true}}]})),
        ]
    }

    #[test]
    fn the_list_is_in_reading_order_and_holds_only_what_can_be_acted_on() {
        let (els, truncated) = elements_from_ax(&page());
        assert!(!truncated);
        let names: Vec<(&str, &str)> = els.iter().map(|e| (e.r#ref.as_str(), e.name.as_str())).collect();
        // childIds order, not listing order: Password is child 4, listed first under 2.
        assert_eq!(names, vec![("e1", "Password"), ("e2", "Email"), ("e3", "Remember me"), ("e4", "Select all"), ("e5", "Sign in")]);
        assert_eq!(els[1].value.as_deref(), Some("a@b.com"));
        assert_eq!(els[2].checked, Some(false), "an unticked checkbox says so");
        assert_eq!(els[3].checked, Some(false), "mixed is not ticked");
        assert!(els[4].disabled);
        assert_eq!(els[4].checked, None, "a button is not checkable");
        assert_eq!(els[0].backend_node_id, 104);
    }

    #[test]
    fn a_huge_page_is_cut_and_says_so() {
        let mut nodes = vec![json!({"nodeId": "0", "role": {"value": "RootWebArea"}, "childIds": (1..=250).map(|i| i.to_string()).collect::<Vec<_>>()})];
        for i in 1..=250 {
            nodes.push(node(&i.to_string(), "link", &format!("Item {i}"), &[], json!({"parentId": "0"})));
        }
        let (els, truncated) = elements_from_ax(&nodes);
        assert_eq!(els.len(), ELEMENT_LIMIT);
        assert!(truncated);
        assert!(render(1, "t", "u", &els, truncated).contains("first 200"));
    }

    #[test]
    fn a_cyclic_tree_does_not_hang() {
        let nodes = vec![
            json!({"nodeId": "1", "role": {"value": "RootWebArea"}, "childIds": ["2"]}),
            node("2", "button", "Loop", &["1"], json!({"parentId": "1"})),
        ];
        let (els, _) = elements_from_ax(&nodes);
        assert_eq!(els.len(), 1);
    }

    #[test]
    fn a_password_value_never_reaches_the_rendered_list() {
        let (mut els, t) = elements_from_ax(&page());
        els[0].input_type = "password".into();
        els[0].value = Some("hunter2".into());
        let text = render(7, "Sign in", "https://example.com/login", &els, t);
        assert!(text.contains("snapshot_id: 7"));
        assert!(text.contains("e1 textbox \"Password\" [password — use browser_request_secret]"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("e2 textbox \"Email\" = \"a@b.com\""), "{text}");
        assert!(text.contains("e5 button \"Sign in\" [disabled]"), "{text}");
    }
