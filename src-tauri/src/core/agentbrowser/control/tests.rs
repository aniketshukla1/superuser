use super::*;

    #[test]
    fn an_ask_does_not_hand_over_control_and_expires_unanswered() {
        let mut c = Control::new(0.0);
        c.request_help("This page wants a code sent to your phone.", 10.0);
        let v = c.view(20.0);
        assert!(v.requested && v.needs_you());
        assert_eq!(v.holder, Holder::Assistant, "asking is not taking");
        assert!(c.bot_may_act().is_ok());
        let later = c.view(10.0 + ASK_TTL_SECS + 1.0);
        assert!(!later.requested && later.reason.is_none(), "an unanswered ask stops being shown");
    }

    #[test]
    fn while_the_user_drives_the_assistant_is_refused_and_the_user_is_never_timed_out() {
        let mut c = Control::new(0.0);
        c.request_help("Sign in, please.", 1.0);
        c.take(2.0);
        assert_eq!(c.bot_may_act(), Err(HUMAN_HAS_CONTROL));
        assert!(c.user_may_drive());
        let v = c.view(2.0 + ASK_TTL_SECS * 5.0);
        assert_eq!(v.holder, Holder::User, "someone halfway through a code keeps the wheel");
        assert_eq!(v.reason.as_deref(), Some("Sign in, please."), "the reason survives the take");
        c.release(3.0);
        assert!(c.bot_may_act().is_ok());
        assert_eq!(c.view(3.0).reason, None, "and goes with the hand-back");
        assert_eq!(c.releases(), 1);
    }

    #[test]
    fn a_secret_is_answerable_only_while_it_is_asked_for() {
        let mut c = Control::new(0.0);
        assert!(c.pending_secret(1.0).is_none(), "nothing asked, nothing accepted");
        c.request_secret("the six-digit code", "e7", 42, 1.0);
        let v = c.view(2.0);
        assert_eq!(v.secret_wanted.as_deref(), Some("the six-digit code"));
        assert_eq!(c.pending_secret(2.0).unwrap().backend_node_id, 42);
        c.secret_supplied();
        assert!(c.pending_secret(3.0).is_none());
        assert_eq!(c.supplied(), 1);

        c.request_secret("", "e7", 42, 10.0);
        assert!(c.pending_secret(10.0 + ASK_TTL_SECS + 1.0).is_none(), "a stale ask stops being answerable");
    }

    #[test]
    fn taking_the_wheel_closes_a_pending_secret() {
        let mut c = Control::new(0.0);
        c.request_secret("password", "e2", 7, 1.0);
        c.take(2.0);
        assert!(c.pending_secret(2.0).is_none());
        assert!(c.view(2.0).secret_wanted.is_none());
    }

    #[test]
    fn a_release_without_a_take_is_not_counted_as_help_given() {
        let mut c = Control::new(0.0);
        c.release(1.0);
        assert_eq!(c.releases(), 0);
    }

    #[test]
    fn not_now_is_an_answer_and_only_to_an_open_request() {
        let mut c = Control::new(0.0);
        assert!(!c.decline(1.0), "nothing asked, nothing to decline");
        c.request_help("Sign in, please.", 1.0);
        assert!(c.decline(2.0));
        assert_eq!((c.declines(), c.releases()), (1, 0));
        assert!(!c.view(2.0).needs_you());
        c.request_help("Sign in, please.", 3.0);
        c.take(4.0);
        assert!(!c.decline(5.0), "a wheel the user holds is not declined out from under them");
        assert_eq!(c.holder(), Holder::User);
    }
