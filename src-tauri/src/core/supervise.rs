//! Keeping the app's own background work alive.
//!
//! The schedulers, the Slack and Telegram watchers and the local API each run as one
//! long-lived task. A panic in any of them used to end that task and nothing else: tokio
//! swallows it, the app keeps its window, and routines quietly stop firing or the chat stops
//! answering until someone restarts the app and never learns why.
//!
//! So each one is started again when it ends, after a wait that doubles each time it fails
//! quickly (1s, 2s, 4s … a minute) and resets once it has run for five minutes — a crash loop
//! must not spin a core, and one crash a day must not leave a minute-long gap. Every restart
//! is counted and kept with its reason, and the health endpoint shows them: a supervisor that
//! restarts things silently is only a quieter way to hide the crash.

use std::future::Future;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

pub const BASE: Duration = Duration::from_secs(1);
pub const MAX: Duration = Duration::from_secs(60);
/// A task that ran this long before ending was working; its next failure starts the count again.
pub const STABLE: Duration = Duration::from_secs(300);

/// How long to wait before starting something again.
#[derive(Debug, Default)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    /// The wait after a run that lasted `ran_for`.
    pub fn after(&mut self, ran_for: Duration) -> Duration {
        if ran_for >= STABLE {
            self.failures = 0;
        }
        let wait = BASE.saturating_mul(1u32 << self.failures.min(16)).min(MAX);
        self.failures = self.failures.saturating_add(1);
        wait
    }
}

/// One supervised task's restarts, as the health endpoint reports them.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Restarts {
    pub name: String,
    pub count: u32,
    pub last_reason: String,
    /// Epoch seconds.
    pub last_at: u64,
}

fn ledger() -> &'static Mutex<Vec<Restarts>> {
    static L: OnceLock<Mutex<Vec<Restarts>>> = OnceLock::new();
    L.get_or_init(Default::default)
}

pub fn record(name: &str, reason: &str) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut l = ledger().lock().unwrap_or_else(|e| e.into_inner());
    match l.iter_mut().find(|r| r.name == name) {
        Some(r) => {
            r.count += 1;
            r.last_reason = reason.to_string();
            r.last_at = now;
        }
        None => l.push(Restarts { name: name.to_string(), count: 1, last_reason: reason.to_string(), last_at: now }),
    }
}

/// Every task that has had to be started again since the app launched.
pub fn restarts() -> Vec<Restarts> {
    ledger().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// What a panic said, as far as it can be read.
pub fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        return s.to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "a panic with no message".into()
}

/// Run the task `make` builds, and build and run it again whenever it ends.
///
/// `make` is called once per run with its own handle, so nothing a run owns outlives it.
pub fn keep_running<F, Fut>(name: &'static str, handle: tauri::AppHandle, make: F)
where
    F: Fn(tauri::AppHandle) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut backoff = Backoff::default();
        loop {
            let began = Instant::now();
            let why = match tokio::spawn(make(handle.clone())).await {
                Ok(()) => "stopped on its own".to_string(),
                Err(e) if e.is_panic() => format!("crashed: {}", panic_text(e.into_panic())),
                Err(_) => "was cancelled".to_string(),
            };
            let wait = backoff.after(began.elapsed());
            record(name, &why);
            eprintln!("[superuser] {name} {why} — starting it again in {}s", wait.as_secs());
            tokio::time::sleep(wait).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_that_keeps_failing_waits_longer_each_time_up_to_a_minute() {
        let mut b = Backoff::default();
        let quick = Duration::from_millis(10);
        let waits: Vec<u64> = (0..9).map(|_| b.after(quick).as_secs()).collect();
        assert_eq!(waits, vec![1, 2, 4, 8, 16, 32, 60, 60, 60]);
        // Hundreds of failures later it is still a minute, not an overflow.
        for _ in 0..200 {
            assert_eq!(b.after(quick), MAX);
        }
    }

    #[test]
    fn a_task_that_ran_a_good_while_starts_the_count_again() {
        let mut b = Backoff::default();
        for _ in 0..5 {
            b.after(Duration::from_millis(1));
        }
        assert_eq!(b.after(STABLE), BASE, "one crash after five minutes of work is not a crash loop");
        assert_eq!(b.after(Duration::from_millis(1)), Duration::from_secs(2));
    }

    #[test]
    fn restarts_are_counted_with_their_latest_reason() {
        record("test-loop-a", "crashed: first");
        record("test-loop-a", "crashed: second");
        let r = restarts().into_iter().find(|r| r.name == "test-loop-a").unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.last_reason, "crashed: second");
        assert_eq!(panic_text(Box::new("boom")), "boom");
        assert_eq!(panic_text(Box::new(String::from("bang"))), "bang");
        assert_eq!(panic_text(Box::new(42)), "a panic with no message");
    }

    #[tokio::test]
    async fn a_panicking_task_is_seen_as_a_crash_with_its_message() {
        let e = tokio::spawn(async { panic!("the scheduler fell over") }).await.unwrap_err();
        assert!(e.is_panic());
        assert_eq!(panic_text(e.into_panic()), "the scheduler fell over");
    }
}
