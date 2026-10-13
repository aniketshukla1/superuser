use super::*;

    #[tokio::test]
    pub(super) async fn an_answer_for_nobody_is_dropped() {
        assert!(!answer("nobody-asked", "hello"));
    }

    #[tokio::test]
    pub(super) async fn asking_again_supersedes_rather_than_queues() {
        let first = park("s2");
        let second = park("s2");
        assert!(answer("s2", "b"));
        // The first ends because its sender was dropped, NOT with a stale answer.
        assert!(first.await.is_err());
        assert_eq!(second.await.unwrap(), "b");
    }

    #[tokio::test]
    pub(super) async fn abandoning_ends_the_wait_without_an_answer() {
        let rx = park("s3");
        abandon("s3");
        assert!(rx.await.is_err());
        assert!(!is_waiting("s3"));
    }
