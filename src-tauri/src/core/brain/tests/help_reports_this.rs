use super::*;

    #[test]
    pub(super) fn help_reports_this_brain_not_a_generic_blurb() {
        let h = help(&census_fixture());
        assert!(h.contains("**42** memories across 2 categories"));
        assert!(h.contains("**9** days of timeline, plus distilled core memory"));
        assert!(h.contains("@brain"));
    }
