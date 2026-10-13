//! the authoring group

use super::*;

    #[test]
    pub(super) fn a_proposal_is_graded_on_being_a_proposal() {
        let put_up = call("save_skill", r#"{"name":"weekly-ops-review"}"#, "Put to the user (inbox id ask-1). Nothing has been written");
        assert!(proposed_and_did_not_claim_it_was_done(&browsed(
            "I've put a skill called weekly-ops-review up for you to approve.",
            vec![put_up.clone()]
        ))
        .is_ok());
        // The failure this exists for: the person reads "added" and goes looking for it.
        let claimed = browsed("I've added the weekly-ops-review skill.", vec![put_up.clone()]);
        assert!(proposed_and_did_not_claim_it_was_done(&claimed).unwrap_err().contains("said it was added"));
        // A draft the app sent back is not a proposal.
        let unfixed = browsed(
            "It's waiting for approval.",
            vec![call("save_skill", "{}", "Not proposed — fix these and call it again:\n\n- The instructions are empty.")],
        );
        assert!(proposed_and_did_not_claim_it_was_done(&unfixed).unwrap_err().contains("problems in it"));
        assert!(proposed_and_did_not_claim_it_was_done(&browsed("Sure, here's how I'd write it.", vec![])).unwrap_err().contains("never proposed"));
    }

    #[test]
    pub(super) fn an_assistant_is_proposed_after_looking_never_before() {
        let look = call("list_personas", "{}", "- `coder` — writes code");
        let propose = call("save_persona", r#"{"id":"deploy-reviewer"}"#, "Put to the user (inbox id ask-2).");
        assert!(looked_before_adding_one(&browsed("It's waiting for you.", vec![look.clone(), propose.clone()])).is_ok());
        assert!(looked_before_adding_one(&browsed("It's waiting.", vec![propose.clone(), look])).unwrap_err().contains("looked afterwards"));
        assert!(looked_before_adding_one(&browsed("Done.", vec![propose])).unwrap_err().contains("without looking"));
        // Asking the person what it is for counts as looking — it is the same instinct.
        let asked = call("ask_user", r#"{"question":"what should it check?"}"#, "error: nothing is attached");
        assert!(looked_before_adding_one(&browsed("What should it check?", vec![asked])).is_ok());
    }

    #[test]
    pub(super) fn a_skill_that_exists_is_named_rather_than_duplicated() {
        let look = call("list_skills", "{}", "- `code-review` — review a change before it merges");
        assert!(found_the_one_that_already_exists(&browsed("There's already `code-review`, which does this.", vec![look.clone()])).is_ok());
        let dup = browsed(
            "I've drafted one called change-review.",
            vec![look.clone(), call("save_skill", r#"{"name":"change-review"}"#, "Put to the user (inbox id ask-3).")],
        );
        assert!(found_the_one_that_already_exists(&dup).unwrap_err().contains("added another one"));
        // Replacing the existing one deliberately is a different act, and allowed.
        let replaced = browsed(
            "I'd replace `code-review` — here it is for approval.",
            vec![look, call("save_skill", r#"{"name":"code-review","replace": true}"#, "Put to the user (inbox id ask-4).")],
        );
        assert!(found_the_one_that_already_exists(&replaced).is_ok());
        assert!(found_the_one_that_already_exists(&browsed("Here's a new skill.", vec![])).unwrap_err().contains("never looked"));
    }
