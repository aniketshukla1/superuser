//! the routine group

use super::*;

    #[test]
    pub(super) fn the_firing_tasks_quote_the_frame_the_app_uses() {
        // The prompt carries the frame verbatim because a task cannot reference a const in
        // a string literal. If the app's wording changes, this is what notices.
        for id in ["a_firing_does_the_work_not_the_scheduling", "a_firing_that_reads_like_a_question_still_does_the_work"] {
            let t = TASKS.iter().chain(HELD_OUT.iter()).find(|t| t.id == id).expect("the task exists");
            assert!(
                t.prompt.starts_with(crate::core::automation::FIRING_FRAME),
                "{id}: its prompt has drifted from the frame the app puts in front of a firing"
            );
        }
    }

    #[test]
    pub(super) fn a_firing_is_graded_on_the_work_landing() {
        let mut did = rec("Appended it.");
        did.marker = "MK0001".into();
        did.files.insert("notes.md".into(), "# Notes\n\nMK0001\n".into());
        did.calls = vec![call("write_file", r#"{"path":"notes.md"}"#, "wrote notes.md")];
        assert!(did_the_work_not_the_scheduling(&did).is_ok());

        // The known bug: the instruction read as a question about the automation.
        let mut listed = did.clone();
        listed.calls = vec![call("list_automations", "{}", "- **notes** — every day")];
        listed.files.insert("notes.md".into(), "# Notes\n\n[start] nothing yet\n".into());
        assert!(did_the_work_not_the_scheduling(&listed).unwrap_err().contains("instead of the work"));

        // It said it did, and the file says otherwise.
        let mut claimed = did.clone();
        claimed.files.insert("notes.md".into(), "# Notes\n\n[start] nothing yet\n".into());
        assert!(did_the_work_not_the_scheduling(&claimed).unwrap_err().contains("didn't land"));
    }
