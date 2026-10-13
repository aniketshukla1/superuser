use super::*;
    use crate::core::agent::RiskClass;
    use crate::core::inbox::{decide, Decision};

    #[test]
    fn fetching_verbs_are_read_class() {
        for name in ["notion_search", "linear-list-issues", "slack.get_channel", "github_read_file", "jira_query_issues", "docs_retrieve", "list", "describe_table"] {
            assert_eq!(mcp_risk(name), RiskClass::Read, "{name} only fetches");
        }
    }

    #[test]
    fn anything_that_could_change_something_is_external() {
        for name in ["notion_create_page", "linear_update_issue", "slack_post_message", "github_merge_pull_request", "delete_page", "send_email", "wibble"] {
            assert_eq!(mcp_risk(name), RiskClass::External, "{name} must be gated");
        }
    }

    #[test]
    fn an_answer_citing_a_link_it_never_opened_goes_back_to_the_model_once() {
        let mut challenged = false;
        let answer = "The full breakdown is at https://acme.io/2026-report.";

        let first = citation_challenge(answer, &[], "chart my sales", &mut challenged);
        assert!(first.unwrap().contains("acme.io/2026-report"));

        // And exactly once. Without the latch a model that re-cites the same link
        // ping-pongs until the step budget runs out, and the user waits through every
        // round of it for an answer they could already read.
        assert!(citation_challenge(answer, &[], "chart my sales", &mut challenged).is_none());
    }

    #[test]
    fn an_answer_with_nothing_to_answer_for_is_delivered_untouched() {
        let mut challenged = false;
        assert!(citation_challenge("Q4 carried the year.", &[], "", &mut challenged).is_none());
        assert!(!challenged, "a clean answer must not spend the one challenge");
        // A link it did open, and a link the user handed it.
        assert!(citation_challenge("see https://acme.io/x", &["https://acme.io/x".into()], "", &mut challenged).is_none());
        assert!(citation_challenge("see https://acme.io/y", &[], "read https://acme.io/y", &mut challenged).is_none());
    }

    #[test]
    fn a_verb_embedded_in_a_word_does_not_make_a_tool_read_only() {
        // "forget" contains "get", "unlisted" contains "list" — matching on
        // substrings instead of whole segments would wave both straight through.
        assert_eq!(mcp_risk("forget_credentials"), RiskClass::External);
        assert_eq!(mcp_risk("unlisted_publish"), RiskClass::External);
    }

    #[test]
    fn write_tools_are_gated_by_mode_and_read_tools_never_are() {
        // Read-class sails through every mode, including the ones that forbid acting.
        for mode in ["discuss", "plan", "interactive", "auto"] {
            assert_eq!(decide("notion_search", "notion", mcp_risk("notion_search"), mode, &[], false), Decision::Allow);
        }
        let write = mcp_risk("notion_create_page");
        assert_eq!(decide("notion_create_page", "notion", write, "discuss", &[], false), Decision::Deny);
        assert_eq!(decide("notion_create_page", "notion", write, "interactive", &[], false), Decision::Ask);
        assert_eq!(decide("notion_create_page", "notion", write, "auto", &[], false), Decision::Allow);
        // A standing rule on the automation pre-approves exactly one tool+target.
        let standing = vec!["notion_create_page notion".to_string()];
        assert_eq!(decide("notion_create_page", "notion", write, "interactive", &standing, false), Decision::Allow);
        assert_eq!(decide("notion_delete_page", "notion", write, "interactive", &standing, false), Decision::Ask);
    }

    #[test]
    fn one_mcp_result_taints_the_run_for_the_next_connected_app_write() {
        // An MCP server's output is a stranger's text, so a run that has read one
        // no longer gets to act on a connected app without a human — even with the
        // standing rule that would otherwise pre-approve it.
        let write = mcp_risk("notion_create_page");
        let standing = vec!["notion_create_page notion".to_string()];
        assert_eq!(decide("notion_create_page", "notion", write, "auto", &standing, true), Decision::Ask);
        // Reading from the app is still fine — that is how the work gets done.
        assert_eq!(decide("notion_search", "notion", mcp_risk("notion_search"), "auto", &[], true), Decision::Allow);
    }
