//! When a failed request is worth sending again — and when sending it again is just
//! noise.
//!
//! HARNESS-PLAN §2.6: a single non-2xx discarded the whole turn. A 429 or a 529
//! `overloaded_error` are both routine and both retryable, and throwing the turn away
//! on one costs every tool call it had already executed — real work, already done,
//! lost to a condition that clears in a second. There was no backoff anywhere in the
//! loop.
//!
//! The decision is a pure function so the policy can be tested without a socket. That
//! split is the same one `StreamAccum` makes and for the same reason: the sharp edges
//! are in the rules, not in the I/O, and rules that need a network to exercise do not
//! get exercised.

use std::time::Duration;

/// How many times a request is sent in total, first attempt included.
pub const MAX_ATTEMPTS: u32 = 3;

/// What to do about a response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Next {
    /// Hand it back — success, or a failure that asking again cannot fix.
    Done,
    /// Send it again after this long.
    RetryIn(Duration),
}

/// Should this status be tried again, and after how long?
///
/// `attempt` is 1-based. `retry_after` is the header's value in seconds when the
/// server sent one — it is the server telling us what it wants, and guessing over the
/// top of that is how a backoff becomes a stampede.
///
/// Deliberately NOT retried: every 4xx except 408 and 429. A 400 is a request the
/// server understood and refused, and sending it again unchanged asks the same
/// question three times to get the same answer — while the user waits.
pub fn decide(status: u16, attempt: u32, retry_after: Option<u64>) -> Next {
    if (200..300).contains(&status) || attempt >= MAX_ATTEMPTS {
        return Next::Done;
    }
    let retryable = matches!(status, 408 | 429 | 500 | 502 | 503 | 504 | 529);
    if !retryable {
        return Next::Done;
    }
    if let Some(secs) = retry_after {
        // Trust it, but not unboundedly: a server asking for ten minutes is telling
        // us to give up, not to hold a turn open that long.
        return if secs > 30 { Next::Done } else { Next::RetryIn(Duration::from_secs(secs)) };
    }
    Next::RetryIn(backoff(attempt))
}

/// Exponential, with jitter. The jitter is not decoration: without it every client
/// that failed at the same moment comes back at the same moment, which is how a
/// provider that was briefly busy is kept busy.
///
/// The first version derived the jitter from `SystemTime`'s sub-second nanos, on the
/// reasoning that a real RNG was a dependency for a number nobody verifies. The test
/// below caught it flat: macOS reports that clock at microsecond granularity, so the
/// value is always a multiple of 1000 and `% 500` is always 0 — a jitter that was
/// exactly zero, every time, on the platform this is developed on. `rand` was already
/// a dependency. Cleverness that needs a test to notice it failed is not cheaper than
/// the crate it avoided.
fn backoff(attempt: u32) -> Duration {
    use rand::Rng;
    let base = 500u64 << (attempt.saturating_sub(1).min(4)); // 500ms, 1s, 2s…
    let jitter = rand::thread_rng().gen_range(0..=base / 4); // up to +25%
    Duration::from_millis(base + jitter)
}

/// `Retry-After` in seconds, when the header is present and is a plain number.
///
/// The HTTP-date form is legal and unused by the providers this app talks to; parsing
/// it would be code no request exercises. An unparseable value falls back to the
/// backoff rather than to zero — "immediately" is the one answer the header can never
/// have meant.
pub fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_routine_failures_are_the_ones_worth_repeating() {
        // 429 and 529 are the two this exists for: both routine, both clearing on
        // their own, and both were costing the whole turn.
        for status in [408, 429, 500, 502, 503, 504, 529] {
            assert!(
                matches!(decide(status, 1, None), Next::RetryIn(_)),
                "{status} should be retried"
            );
        }
    }

    #[test]
    fn a_refusal_is_not_retried_however_many_attempts_are_left() {
        // A 400 is a request the server understood and refused. Asking again
        // unchanged gets the same answer, three times, while the user waits.
        for status in [400, 401, 403, 404, 409, 413, 422] {
            assert_eq!(decide(status, 1, None), Next::Done, "{status} must not be retried");
        }
    }

    #[test]
    fn success_stops_immediately() {
        for status in [200, 201, 204, 299] {
            assert_eq!(decide(status, 1, None), Next::Done);
        }
    }

    #[test]
    fn attempts_are_capped_so_a_bad_minute_does_not_become_a_bad_turn() {
        assert!(matches!(decide(529, 1, None), Next::RetryIn(_)));
        assert!(matches!(decide(529, 2, None), Next::RetryIn(_)));
        assert_eq!(decide(529, MAX_ATTEMPTS, None), Next::Done, "the cap is a cap");
    }

    #[test]
    fn the_server_is_believed_when_it_names_a_delay() {
        // It knows when it will be ready and we do not; guessing over the top of a
        // `Retry-After` is how a backoff becomes a stampede.
        assert_eq!(decide(429, 1, Some(2)), Next::RetryIn(Duration::from_secs(2)));
        assert_eq!(decide(429, 1, Some(0)), Next::RetryIn(Duration::from_secs(0)));
        // Except when what it names is "not soon". Holding a turn open for ten
        // minutes is not a retry, it is a hang with a reason.
        assert_eq!(decide(429, 1, Some(600)), Next::Done);
    }

    #[test]
    fn waits_grow_and_never_collide_exactly() {
        // Growth, so a provider under load is not hammered at a fixed rate…
        let a = backoff(1);
        let c = backoff(3);
        assert!(c > a, "backoff must grow: {a:?} then {c:?}");
        // …and jitter, so two clients that failed together do not return together.
        // Sampled, because two draws CAN legitimately match.
        let draws: std::collections::HashSet<u128> =
            (0..40).map(|_| backoff(3).as_nanos()).collect();
        assert!(draws.len() > 1, "every backoff was identical — the jitter is not jittering");
    }

    #[test]
    fn a_header_we_cannot_read_falls_back_rather_than_meaning_now() {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(reqwest::header::RETRY_AFTER, "3".parse().unwrap());
        assert_eq!(retry_after_secs(&h), Some(3));

        // The HTTP-date form is legal and unused here; it must not read as 0.
        h.insert(reqwest::header::RETRY_AFTER, "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap());
        assert_eq!(retry_after_secs(&h), None);
        assert!(matches!(decide(429, 1, retry_after_secs(&h)), Next::RetryIn(_)));

        assert_eq!(retry_after_secs(&reqwest::header::HeaderMap::new()), None);
    }
}
