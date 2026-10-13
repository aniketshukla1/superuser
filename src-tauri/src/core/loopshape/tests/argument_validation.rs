//! argument validation

use super::*;

    #[test]
    pub(super) fn a_missing_required_argument_is_named_before_the_tool_runs() {
        // The live shape of this: `write_file` with no path dispatched anyway, the
        // tool returning some unrelated error, and the model spending its next step
        // debugging a problem it does not have.
        let schema = json!({
            "type": "object",
            "properties": { "path": { "type": "string" }, "content": { "type": "string" } },
            "required": ["path", "content"]
        });
        let err = check_args(&schema, &json!({ "content": "hi" })).unwrap_err();
        assert!(err.contains("path"), "got: {err}");
        // And it says what a correct call looks like, so the retry is informed.
        assert!(err.contains("path (string)") && err.contains("content (string)"));
        assert!(check_args(&schema, &json!({ "path": "a.md", "content": "hi" })).is_ok());
    }

    #[test]
    pub(super) fn an_argument_of_the_wrong_type_is_caught_with_both_types_named() {
        let schema = json!({
            "type": "object",
            "properties": { "command": { "type": "string" }, "limit": { "type": "integer" } },
            "required": ["command"]
        });
        let err = check_args(&schema, &json!({ "command": ["ls", "-a"] })).unwrap_err();
        assert!(err.contains("must be string") && err.contains("got array"), "got: {err}");
        // 2.5 is a number and is not an integer.
        assert!(check_args(&schema, &json!({ "command": "ls", "limit": 2.5 })).is_err());
        assert!(check_args(&schema, &json!({ "command": "ls", "limit": 2 })).is_ok());
    }

    #[test]
    pub(super) fn a_null_is_a_missing_argument_and_not_a_present_one() {
        // Models emit `null` for "I have nothing for this" routinely. Counting the key
        // as present sends the tool a null it will fail on a layer deeper down.
        let schema = json!({ "properties": { "path": { "type": "string" } }, "required": ["path"] });
        assert!(check_args(&schema, &json!({ "path": null })).is_err());
    }

    #[test]
    pub(super) fn what_the_validator_does_not_know_it_does_not_reject() {
        // Half a JSON Schema implementation that claims to be one is worse than a
        // partial one that says so. An unconstrained property, an unknown type
        // keyword and an extra argument all pass — the last is §2.17's job, not this.
        let schema = json!({
            "type": "object",
            "properties": { "anything": {}, "odd": { "type": "null" } },
            "required": []
        });
        assert!(check_args(&schema, &json!({ "anything": [1, 2], "odd": 7, "extra": true })).is_ok());
        // A schema with no `properties` at all constrains nothing.
        assert!(check_args(&json!({}), &json!({ "whatever": 1 })).is_ok());
    }

    #[test]
    pub(super) fn arguments_that_are_not_an_object_are_refused_by_shape() {
        // The `{}` fallback H3 replaced could at least be indexed. A bare string or
        // array reaching a tool that expects named arguments cannot.
        assert!(check_args(&json!({"required": ["p"]}), &json!("just a string")).is_err());
        assert!(check_args(&json!({}), &json!([1, 2, 3])).is_err());
    }
