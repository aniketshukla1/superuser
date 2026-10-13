use super::*;
    use serde_json::json;

    #[test]
    fn a_credential_is_found_by_its_name_or_by_its_shape() {
        let found = inspect(&json!({"title": "notes", "password": "hunter2"}));
        assert_eq!(found, vec![Finding { kind: "a password", at: "password".into() }]);

        let pem = inspect(&json!({"body": "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----"}));
        assert_eq!(pem[0].kind, "a private key");

        let header = inspect(&json!({"headers": {"Authorization": "Bearer abcdefghijklmnopqrstuvwxyz"}}));
        assert!(header.iter().any(|f| f.kind == "an authorization header" && f.at == "headers.Authorization"));

        // A provider token pasted into prose, where nothing is named after it.
        let pasted = inspect(&json!({"comment": "the build used AKIAIOSFODNN7EXAMPLE and failed"}));
        assert_eq!(pasted[0].kind, "what looks like a provider token");
        assert_eq!(pasted[0].at, "comment");
    }

    #[test]
    fn a_finding_never_carries_the_secret_itself() {
        let found = inspect(&json!({"page": {"blocks": [{"text": "sk-livetokenthatshouldnotleak00"}]}}));
        assert_eq!(found[0].at, "page.blocks[0].text", "the path is what you decide with");
        let said = refusal("notion", &found);
        assert!(!said.contains("sk-livetokenthatshouldnotleak00"), "the refusal must not repeat it");
        assert!(said.contains("Refused.") && said.contains("was not sent"));
    }

    #[test]
    fn ordinary_arguments_are_left_alone() {
        assert!(inspect(&json!({"title": "Q3 review", "rows": [{"quarter": "Q3", "revenue": 1850}]})).is_empty());
        // Deliberately not general PII: a name and an address are not this scanner's claim.
        assert!(inspect(&json!({"to": "Ada Lovelace", "address": "12 Mill Lane"})).is_empty());
        // A field CALLED token but empty is nothing to refuse.
        assert!(inspect(&json!({"token": ""})).is_empty());
        // A short word beginning `sk-` is not a key.
        assert!(inspect(&json!({"note": "sk-12"})).is_empty());
    }

    #[test]
    fn it_fails_closed_when_it_cannot_see_the_whole_call() {
        let deep = (0..MAX_DEPTH + 5).fold(json!("x"), |acc, _| json!({ "inner": acc }));
        assert!(inspect(&deep).iter().any(|f| f.kind.contains("nested too deeply")));

        let wide = json!({ "rows": (0..MAX_NODES + 50).map(|i| json!({ "i": i })).collect::<Vec<_>>() });
        assert!(inspect(&wide).iter().any(|f| f.kind.contains("too large to check")));

        let long = json!({ "body": "x".repeat(MAX_STRING + 1) });
        assert!(inspect(&long).iter().any(|f| f.kind.contains("too long to check")));
    }

    #[test]
    fn a_connected_app_lives_somewhere_a_person_can_check() {
        assert_eq!(mcp_endpoint_problem("https://mcp.example.com/v1"), None);
        assert!(mcp_endpoint_problem("http://mcp.example.com").unwrap().contains("plain http"));
        assert!(mcp_endpoint_problem("ftp://mcp.example.com").unwrap().contains("https://"));
        assert!(mcp_endpoint_problem("https://user:pw@mcp.example.com").unwrap().contains("username or password"));
        assert!(mcp_endpoint_problem("https://mcp.example.com/v1?api_key=abc").unwrap().contains("api_key"));
        assert!(mcp_endpoint_problem("https://mcp.example.com/v1#token=abc").unwrap().contains("token"));
        assert!(mcp_endpoint_problem("https://localhost:3000/mcp").unwrap().contains("this machine"));
        // Every IP literal is refused as one — including the ones that are also this
        // machine's own network, where either sentence would be true.
        assert!(mcp_endpoint_problem("https://169.254.169.254/mcp").unwrap().contains("bare IP"));
        assert!(mcp_endpoint_problem("https://203.0.113.5/mcp").unwrap().contains("bare IP"));
        assert!(mcp_endpoint_problem("https://mcp/v1").unwrap().contains("no domain"));
        assert!(mcp_endpoint_problem("https://tools.internal/mcp").unwrap().contains("private network"));
        assert!(mcp_endpoint_problem("   ").unwrap().contains("give the server's address"));
    }

    #[test]
    fn access_taken_away_is_told_apart_from_a_call_that_went_wrong() {
        assert!(is_revoked("HTTP 401 Unauthorized"));
        assert!(is_revoked("{\"error\":\"invalid_token\"}"));
        assert!(is_revoked("the refresh token has expired"));
        assert!(!is_revoked("connection reset by peer"));
        assert!(!is_revoked("500 Internal Server Error"));
        let said = revoked("Linear");
        assert!(said.contains("Connected apps") && said.contains("Don't retry"));
    }

    #[test]
    fn three_kinds_of_answer_never_read_alike() {
        assert!(vendor_error("caller does not have permission").starts_with("The app reported an error:"));
        assert!(call_failed("connection reset").starts_with("That tool could not be called:"));
        let empty = empty_result("linear", "list_issues");
        assert!(empty.contains("returned nothing at all") && empty.contains("not the same as"));
        // Newlines collapse: a stack trace in a tool result should not become the answer.
        assert!(!vendor_error("line one\nline two").contains('\n'));
    }
