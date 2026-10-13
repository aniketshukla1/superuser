use super::*;

#[test]
    pub(super) fn a_form_asks_for_values_and_reports_the_blanks() {
        let f = Form::parse(&json!({
            "title": "Model assumptions",
            "intro": "Two numbers I cannot look up.",
            "fields": [
                {"name": "margin", "label": "Target margin", "kind": "number"},
                {"name": "start", "label": "Start date", "kind": "date", "required": false}
            ]
        }))
        .unwrap();
        assert_eq!(f.summary(), "Model assumptions");
        assert_eq!(f.fields[0].kind, "number");
        assert!(f.fields[0].required, "required unless it says otherwise");
        assert!(!f.fields[1].required);

        let out = f.read_answer(r#"{"answers":{"margin":"32%","start":""}}"#);
        assert!(out.contains("Target margin → 32%"), "got: {out}");
        // A blank is reported AS blank. Dropping it hands back a shorter list than was
        // asked, which reads as "that field didn't happen" — the gap a model fills in.
        assert!(out.contains("Start date → (left blank)"), "got: {out}");
        assert!(out.contains("Do not invent a value"), "got: {out}");
    }

    #[test]
    pub(super) fn a_field_name_is_derived_when_the_model_omits_it() {
        // The name is only a key. A model that wrote a good label has said enough,
        // and burning a turn on a reformat helps nobody.
        let f = Form::parse(&json!({"fields": [{"label": "Target margin (%)"}]})).unwrap();
        assert_eq!(f.fields[0].name, "target_margin");
        assert_eq!(f.fields[0].kind, "text", "an unstated kind is text, not a refusal");
    }

    #[test]
    pub(super) fn an_empty_form_points_at_the_tool_that_should_have_been_used() {
        let err = Form::parse(&json!({"fields": []})).unwrap_err();
        assert!(err.contains("ask_user"), "a form with nothing in it is usually a decision: {err}");
        let err = Form::parse(&json!({"fields": [{"placeholder": "no label here"}]})).unwrap_err();
        assert!(err.contains("label"), "got: {err}");
    }

    #[test]
    pub(super) fn a_form_of_ten_fields_is_an_intake_form_and_is_refused() {
        let many: Vec<_> = (0..10).map(|i| json!({"label": format!("Field {i}")})).collect();
        let err = Form::parse(&json!({"fields": many})).unwrap_err();
        assert!(err.contains("intake form"), "got: {err}");
    }

    #[test]
    pub(super) fn a_prose_reply_to_a_form_is_still_an_answer() {
        // Answered from the Inbox, or by an older client. Their words are their answer.
        let f = Form::parse(&json!({"fields": [{"label": "Target margin"}]})).unwrap();
        let out = f.read_answer("about thirty percent");
        assert!(out.contains("about thirty percent"), "got: {out}");
    }

    pub(super) fn one() -> Value {
        json!({
            "question": "Where is this initiative right now?",
            "options": [
                { "label": "Still pre-presentation", "description": "The deck hasn't been shown to stakeholders yet." },
                { "label": "Presented, now piloting", "description": "Leadership has seen it; phase 1 is executing." }
            ]
        })
    }

    #[test]
    pub(super) fn a_question_needs_something_to_answer() {
        assert!(Ask::parse(&json!({ "question": "   ", "options": ["a", "b"] })).is_err());
        assert!(Ask::parse(&json!({})).is_err());
        assert_eq!(Ask::parse(&one()).unwrap().questions[0].question, "Where is this initiative right now?");
    }

    #[test]
    pub(super) fn a_question_without_likely_answers_is_refused() {
        // The whole point of the rebuild: a blank box moves the thinking from the
        // agent — which has just read the files — onto the user.
        let bare = Ask::parse(&json!({ "question": "Which sheet?" })).unwrap_err();
        assert!(bare.contains("fewer than 2 options"), "got: {bare}");
        assert!(bare.contains("look first"), "the refusal has to name the way out: {bare}");
        assert!(bare.contains("Other"), "it must say the escape hatch is free: {bare}");
        assert!(Ask::parse(&json!({ "question": "Which sheet?", "options": ["Q3"] })).is_err());
    }

    #[test]
    pub(super) fn a_bare_string_option_is_accepted_rather_than_bounced() {
        // Models reach for the simple shape constantly; a reformat costs a whole turn.
        let a = Ask::parse(&json!({ "question": "Which quarter?", "options": ["Q3", "Q4"] })).unwrap();
        assert_eq!(a.questions[0].options[0].label, "Q3");
        assert_eq!(a.questions[0].options[0].description, "");
    }

    #[test]
    pub(super) fn descriptions_survive_because_they_are_what_makes_it_answerable() {
        let a = Ask::parse(&one()).unwrap();
        assert_eq!(a.questions[0].options[1].description, "Leadership has seen it; phase 1 is executing.");
    }

    #[test]
    pub(super) fn an_interrogation_is_trimmed_at_both_ends() {
        let many: Vec<Value> = (0..9)
            .map(|i| json!({ "question": format!("q{i}"), "options": ["a", "b", "c", "d", "e", "f"] }))
            .collect();
        let a = Ask::parse(&json!({ "questions": many })).unwrap();
        assert_eq!(a.questions.len(), MAX_QUESTIONS);
        assert_eq!(a.questions[0].options.len(), MAX_OPTIONS, "the number keys run out, and so does scanning");
    }

    #[test]
    pub(super) fn one_malformed_question_fails_the_call_rather_than_vanishing() {
        // A silently shortened ask is one the model believes it made in full.
        let mixed = json!({ "questions": [
            { "question": "Which quarter?", "options": ["Q3", "Q4"] },
            { "question": "Which sheet?" }
        ]});
        assert!(Ask::parse(&mixed).is_err());
    }

    #[test]
    pub(super) fn the_summary_says_how_many_are_coming() {
        assert_eq!(Ask::parse(&one()).unwrap().summary(), "Where is this initiative right now?");
        let two = json!({ "questions": [one(), { "question": "Which teams first?", "options": ["A", "B"] }] });
        assert!(Ask::parse(&two).unwrap().summary().contains("(2 questions)"));
    }

    /// The keys the card actually reads, pinned by name.
    ///
    /// The version before this shipped a field as `kind` while the card read `type`.
    /// Nothing errored — a dropdown simply rendered as a text box. Unit tests on both
    /// sides passed, because each supplied its own fixture; only the contract between
    /// them was wrong, so only a test of the contract can hold it.
    #[test]
    pub(super) fn the_wire_shape_is_what_the_card_reads() {
        let p = Ask::parse(&one()).unwrap().payload();
        let q = &p["questions"][0];
        for key in ["question", "options", "multi"] {
            assert!(!q[key].is_null(), "the card reads `{key}` and it is not on the wire: {q}");
        }
        for key in ["label", "description"] {
            assert!(!q["options"][0][key].is_null(), "the card reads option.`{key}`: {q}");
        }
    }

    #[test]
    pub(super) fn the_answers_come_back_paired_with_what_was_asked() {
        let a = Ask::parse(&json!({ "questions": [
            { "question": "Which quarter?", "options": ["Q3", "Q4"] },
            { "question": "Which sheet?", "options": ["Revenue", "Costs"] }
        ]}))
        .unwrap();
        let out = a.read_answer(r#"{"answers":["Q4","Revenue"]}"#);
        assert!(out.contains("1. Which quarter? → Q4"), "got: {out}");
        assert!(out.contains("2. Which sheet? → Revenue"), "got: {out}");
    }

    #[test]
    pub(super) fn a_skipped_question_is_reported_as_skipped() {
        // The gap is exactly where a model fills in its own guess.
        let a = Ask::parse(&one()).unwrap();
        let out = a.read_answer(r#"{"answers":[null]}"#);
        assert!(out.contains("(skipped)"), "got: {out}");
        assert!(out.contains("Do not fill those in yourself"), "got: {out}");
    }

    #[test]
    pub(super) fn a_plain_reply_is_still_an_answer() {
        // Answered from the Inbox, or by an older client: prose, not a card reply.
        let a = Ask::parse(&one()).unwrap();
        assert!(a.read_answer("we're doing both, actually").contains("we're doing both"));
    }

    #[tokio::test]
    pub(super) async fn an_answer_reaches_the_waiting_run() {
        let rx = park("s1");
        assert!(is_waiting("s1"));
        assert!(answer("s1", "the second sheet"));
        assert_eq!(rx.await.unwrap(), "the second sheet");
        assert!(!is_waiting("s1"), "answering clears the slot");
    }
