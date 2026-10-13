//! eligibility

use super::*;

    #[test]
    pub(super) fn an_arg_executor_can_never_be_vouched_for() {
        for cmd in ["xargs rm", "timeout 5 curl example.com", "docker run alpine sh"] {
            assert!(!prefix_eligible(&tok(cmd)), "should be ineligible: {cmd}");
        }
    }

    #[test]
    pub(super) fn an_interpreter_is_only_ineligible_when_it_carries_source() {
        assert!(!prefix_eligible(&tok("python -c print(1)")));
        assert!(!prefix_eligible(&tok("node -e require('fs')")));
        assert!(prefix_eligible(&tok("python script.py")));
    }

    #[test]
    pub(super) fn find_exec_is_the_case_this_list_exists_for() {
        // No separator, so splitting never sees it; a `find` prefix rule would
        // otherwise auto-run the `rm`.
        assert!(!prefix_eligible(&tok("find . -name *.tmp -exec rm {} +")));
        assert!(!auto_allowable("find . -exec rm {} +", &["find".to_string()]));
        assert!(auto_allowable("find . -name *.tmp", &["find".to_string()]));
    }
