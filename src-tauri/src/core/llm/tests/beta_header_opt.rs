use super::*;

    #[test]
    pub(super) fn the_beta_header_is_opt_in_rather_than_everyones() {
        // Computer use needs one; nothing else does, and sending it from the shared
        // client would quietly enable a beta for three callers that never asked.
        assert_eq!(LlmRequest::new("claude-opus-5", 10).beta("computer-use-2025-01-24").beta, Some("computer-use-2025-01-24".into()));
        assert_eq!(LlmRequest::new("claude-opus-5", 10).beta, None);
    }
