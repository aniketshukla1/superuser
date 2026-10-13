//! Who gets the local model when two things want it.
//!
//! # The problem this exists for
//!
//! A local runner serves one model at a time. Ollama holds a model in memory,
//! answers requests against it, and evicts it when something else needs the space
//! or when its `keep_alive` runs out. That is fine with one consumer and actively
//! hostile with several, because the cost of being wrong is not a queue — it is a
//! reload.
//!
//! This app has several. The user's chat runs one model. The subconscious sweeps
//! every fifteen minutes, self-evolution reviews after ten idle minutes, the
//! nightly consolidation runs at 3am, profile learning runs on its own clock, and
//! the connector sync embeds on a third. None of them asked the user's permission
//! and none of them knew the user was mid-sentence.
//!
//! Worse, they do not all want the *same* model. Chat follows `cloud_model`, which
//! on a local setup is typically a `custom:` id pointing at loopback; background
//! work follows `ollama_model`; embeddings follow `embed_model`. Three models, one
//! machine's worth of RAM.
//!
//! The reload itself is cheap — measured on the owner's M1 Pro, 2026-09-07, about
//! **2s** against 0.50s warm. An earlier note here said 18–57s; that was measured on
//! a contended machine and is wrong. So this module is NOT justified by reload cost
//! alone, and saying otherwise would be selling it on a number that does not hold.
//!
//! What justifies it is contention, which is the larger and less tidy cost. A local
//! runner serves one sequence at a time (`-np 1` by default), so a background sweep
//! does not politely queue behind the user's turn — the user's turn queues behind
//! *it*, for however long a sweep of the vault takes. Turns of 30–200s are ordinary
//! here, and waiting out someone else's is the difference between an app that feels
//! alive and one that feels broken. There is also a second-order effect worth
//! naming: `keep_alive` is a property of the MODEL, not of the caller, so any
//! consumer that omits it resets the TTL for everyone. Background work reaching for
//! a *different* model evicts the one the user is mid-conversation with.
//!
//! # What this does
//!
//! Foreground turns announce themselves with [`foreground`]. Background work asks
//! [`wait_for_quiet`] before it starts, and is told to wait until no turn is in
//! flight *and* a cooldown has passed since the last one ended.
//!
//! The cooldown is the part that matters and the part that is easy to leave out.
//! Releasing background work the instant a turn finishes is nearly as bad as not
//! gating at all, because the moment a turn finishes is the moment the user starts
//! reading — and they will very often type again inside a minute. Waiting out that
//! window is what turns "the model is usually loaded" into "the model is loaded
//! when you need it".
//!
//! # What this deliberately is not
//!
//! Not a lock, and not a queue. Background work is *delayed*, never blocked: a
//! caller that waits will eventually run, and a caller that does not call this is
//! not prevented from anything. A hard lock around the endpoint would let one stuck
//! background job hold the chat hostage, which is a worse failure than the one
//! being fixed.
//!
//! It is also not applied inside [`crate::core::model_router::complete`], though
//! that is the obvious single choke point. Two callers reach it from *inside* a
//! foreground turn — `context::summarize_dropped` and the reviewer — and a gate
//! there would have a turn waiting for itself to finish. Gating at the schedulers
//! instead makes that deadlock unrepresentable rather than merely avoided: an
//! in-turn caller never goes near one.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long after a turn ends before background work may start.
///
/// Tuned to how people actually use a chat rather than to anything about the
/// model: the gap between an answer arriving and the follow-up being sent is
/// usually shorter than this. Long enough to cover reading an answer and typing a
/// reply; short enough that a genuinely idle machine still gets its housekeeping
/// done many times an hour.
const COOLDOWN: Duration = Duration::from_secs(90);

/// Longest a background caller sleeps before re-checking. Only an upper bound on
/// how stale [`quiet_in`]'s answer may get — a turn that starts while someone is
/// sleeping is caught on the next look.
const MAX_SLEEP: Duration = Duration::from_secs(5);

fn in_flight() -> &'static AtomicUsize {
    static N: AtomicUsize = AtomicUsize::new(0);
    &N
}

fn last_end() -> &'static Mutex<Option<Instant>> {
    static T: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(None))
}

/// Held for the life of a foreground turn. Background work defers while it lives,
/// and for [`COOLDOWN`] after it is dropped.
///
/// Counted rather than boolean, so two chats running at once both have to finish,
/// and so a subagent nested inside a turn cannot release its parent's claim by
/// finishing first. Released in `Drop`, which is what makes it correct across the
/// early returns and the `?` operators the turn is full of — an explicit
/// `end()` call would be skipped by exactly the failure paths where a stuck
/// background job hurts most.
pub struct Foreground(());

impl Drop for Foreground {
    fn drop(&mut self) {
        in_flight().fetch_sub(1, Ordering::SeqCst);
        if let Ok(mut t) = last_end().lock() {
            *t = Some(Instant::now());
        }
    }
}

/// Claim the local model for a turn the user is waiting on.
#[must_use = "the claim lasts only as long as the guard is held"]
pub fn foreground() -> Foreground {
    in_flight().fetch_add(1, Ordering::SeqCst);
    Foreground(())
}

/// Whether a foreground turn is in flight right now.
pub fn busy() -> bool {
    in_flight().load(Ordering::SeqCst) > 0
}

/// How long until background work may start. Zero means now.
fn quiet_in() -> Duration {
    if busy() {
        return MAX_SLEEP;
    }
    match last_end().lock().ok().and_then(|t| *t) {
        Some(end) => COOLDOWN.saturating_sub(end.elapsed()),
        // Nothing has run yet this session, so nothing is being waited on.
        None => Duration::ZERO,
    }
}

/// Wait until the user is not using the model. Call before background model work.
pub async fn wait_for_quiet() {
    loop {
        let wait = quiet_in();
        if wait.is_zero() {
            return;
        }
        tokio::time::sleep(wait.min(MAX_SLEEP)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate is process-global by design — there is one local model — so the
    /// tests for it cannot run beside each other. Taken by every test here, and
    /// deliberately not a `OnceLock<Mutex<()>>` dance at each call site: a poisoned
    /// lock from a failing assertion should not cascade into unrelated failures
    /// that hide it.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        static L: Mutex<()> = Mutex::new(());
        L.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The counter has to survive nesting, because a subagent turn takes its own
    /// guard inside its parent's. If the inner drop cleared the claim, background
    /// work would start under a turn that is still going — which is the exact
    /// failure this module exists to prevent, reintroduced by a bool.
    #[test]
    fn a_nested_turn_does_not_release_its_parents_claim() {
        let _s = serial();
        let outer = foreground();
        assert!(busy());
        {
            let _inner = foreground();
            assert!(busy());
        }
        assert!(busy(), "the parent turn is still running");
        drop(outer);
        assert!(!busy());
    }

    /// Quiet is not "the turn ended", it is "the turn ended a while ago". Asserted
    /// because releasing on the drop alone is the tempting simplification, and it
    /// hands the model away at the precise moment the user starts reading.
    #[test]
    fn the_cooldown_outlasts_the_turn_that_set_it() {
        let _s = serial();
        drop(foreground());
        let wait = quiet_in();
        assert!(!wait.is_zero(), "background work must not start the instant a turn ends");
        assert!(wait <= COOLDOWN);
    }

    /// A turn in flight defers background work regardless of the cooldown clock.
    #[test]
    fn work_defers_while_a_turn_is_running() {
        let _s = serial();
        let _turn = foreground();
        assert!(!quiet_in().is_zero());
    }
}
