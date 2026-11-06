use super::*;
    use serde_json::json;

    fn ctx() -> Json {
        json!({
            "intent": "activate",
            "tool": {"name": "browser_click"},
            "element": {"ref": "e13", "role": "button", "name": "Submit order", "type": ""},
            "page": {"url": "https://shop.example.com/cart", "host": "shop.example.com"},
            "key": "",
            "command": "",
            "file": {"path": "", "name": "", "extension": ""},
            "mcp": {"server": "", "tool": "", "effect": ""},
            "tainted": false,
            "count": 3
        })
    }

    fn yes(src: &str) -> bool {
        parse(src).unwrap_or_else(|e| panic!("{src}: {e}")).test(&ctx()).unwrap_or_else(|e| panic!("{src}: {e}"))
    }

    #[test]
    fn rules_with_nesting_and_lists_read_as_written() {
        // Rules the way a person writes them on the Boundaries page, with grouping, lists
        // and functions. If these stop parsing, saved rules stop meaning anything.
        assert!(yes(r#"(tool.name in ["browser_click", "browser_key"] && contains(element.name, "order")) || (intent == "type" && key == "Enter")"#));
        assert!(!yes(r#"intent == "navigate" && startsWith(page.host, "mail.")"#));
        assert!(!yes(r#"(contains(page.host, "bank") || endsWith(page.host, ".gov")) && intent == "activate""#));
        assert!(yes("true"));
    }

    #[test]
    fn text_matching_ignores_case_so_the_capitalised_button_is_caught() {
        assert!(yes(r#"contains(element.name, "SUBMIT")"#));
        assert!(yes(r#"element.name.contains("submit")"#), "the method spelling means the same");
        assert!(yes(r#"matches(page.host, "^SHOP\\.")"#));
        assert!(yes(r#"page.host.endsWith(".EXAMPLE.com")"#));
        assert!(yes(r#"startsWith(page.url, "HTTPS://")"#));
        // Equality stays exact: `intent == "Activate"` is a typo, not a synonym.
        assert!(!yes(r#"intent == "Activate""#));
    }

    #[test]
    fn a_rule_that_answers_something_other_than_true_or_false_is_broken() {
        // Somebody reading the deny list as a list of labels types a label. It parses,
        // and it must not count as "no".
        let p = parse(r#""Submit order""#).unwrap();
        let err = p.test(&ctx()).unwrap_err();
        assert!(err.message.contains("true or false"), "{err}");
    }

    #[test]
    fn an_unknown_field_is_an_error_not_a_quiet_false() {
        let err = parse("elemnt.name == \"x\"").unwrap().test(&ctx()).unwrap_err();
        assert!(err.message.contains("elemnt"), "{err}");
    }

    #[test]
    fn and_and_or_absorb_an_error_the_other_side_already_decides() {
        // CEL's rule, and the reason is order-independence: the answer must not
        // depend on which way round the rule was written.
        assert!(!yes("false && nosuch == 1"));
        assert!(!yes("nosuch == 1 && false"));
        assert!(yes("true || nosuch == 1"));
        assert!(yes("nosuch == 1 || true"));
        assert!(parse("true && nosuch == 1").unwrap().test(&ctx()).is_err());
        assert!(parse("nosuch == 1 || false").unwrap().test(&ctx()).is_err());
    }

    #[test]
    fn in_lists_and_comparisons_work_on_the_types_they_should() {
        assert!(yes(r#"intent in ["type", "activate"]"#));
        assert!(!yes(r#"intent in ["send"]"#));
        assert!(yes("count > 2 && count <= 3"));
        assert!(yes("size(element.name) == 12"));
        assert!(yes(r#"count == 3.0"#), "an integer and a decimal compare by value");
        assert!(yes(r#"tainted == false"#));
        assert!(yes(r#"!tainted"#));
        assert!(yes(r#"(count > 1 ? "many" : "one") == "many""#));
        assert!(yes(r#"lower(element.name) == "submit order""#));
        assert!(parse(r#"intent in "activate""#).unwrap().test(&ctx()).is_err(), "in on text is a mistake worth naming");
    }

    #[test]
    fn a_bad_pattern_is_a_broken_rule_not_a_miss() {
        let err = parse(r#"matches(page.host, "(")"#).unwrap().test(&ctx()).unwrap_err();
        assert!(err.message.contains("pattern"), "{err}");
    }

    #[test]
    fn syntax_errors_say_what_and_where() {
        let e = parse(r#"intent = "send""#).unwrap_err();
        assert!(e.message.contains("=="), "{e}");
        assert_eq!(e.at, Some(7));

        let e = parse(r#"contains(element.name, "x""#).unwrap_err();
        assert!(e.message.contains("`)`"), "{e}");

        let e = parse(r#"intent == "send" &"#).unwrap_err();
        assert!(e.message.contains("&&"), "{e}");

        let e = parse(r#"intent == "send"#).unwrap_err();
        assert!(e.message.contains("closing quote"), "{e}");

        let e = parse("intent ==").unwrap_err();
        assert!(e.message.contains("ends before"), "{e}");

        let e = parse(r#"intent == "a" "b""#).unwrap_err();
        assert!(e.message.contains("should have ended"), "{e}");

        let e = parse(r#"a == b == c"#).unwrap_err();
        assert!(e.message.contains("&&"), "{e}");

        assert!(parse("   ").is_err());
        assert!(parse(&"x".repeat(MAX_LEN + 1)).is_err());
    }

    #[test]
    fn nesting_is_bounded_so_a_rule_cannot_exhaust_the_stack() {
        let deep = format!("{}true{}", "(".repeat(500), ")".repeat(500));
        let e = parse(&deep).unwrap_err();
        assert!(e.message.contains("nests too deeply"), "{e}");
        let bangs = format!("{}true", "!".repeat(500));
        assert!(parse(&bangs).is_err());
    }

    #[test]
    fn unknown_functions_name_the_ones_that_exist() {
        let e = parse("containz(intent, \"a\")").unwrap().test(&ctx()).unwrap_err();
        assert!(e.message.contains("contains, matches"), "{e}");
    }

    #[test]
    fn raw_strings_and_escapes_read_as_written() {
        assert!(yes(r#"matches(page.url, r"^https://shop\.example\.com/")"#));
        // An unknown escape inside ordinary quotes is kept, so `"\d"` is still a digit
        // class to `matches` rather than a lexing error.
        assert!(yes(r#"matches("order 42", "\d+")"#));
        assert!(yes(r#"'it\'s' == "it's""#));
    }

    #[test]
    fn fields_lists_every_path_a_rule_reads() {
        let p = parse(r#"intent == "activate" && contains(element.name, "x") || page.host.endsWith(".com")"#).unwrap();
        let names: Vec<String> = p.fields().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["intent", "element.name", "page.host"]);
    }
