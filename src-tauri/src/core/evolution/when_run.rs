//! when to run


/// Evolve once a session's context passes this fraction of its budget, even if the
/// turn threshold hasn't been reached. A conversation that long has plenty to
/// review, and it is about to start losing its early turns to trimming anyway.
pub(super) const CONTEXT_PRESSURE: f64 = 0.8;

/// Why a session is (or isn't) due for review. Returned rather than a bare bool so
/// the scheduler can log something meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// Enough turns have accumulated since the last pass.
    Turns,
    /// The conversation is long enough that trimming is imminent.
    ContextPressure,
    NotIdleYet,
    NothingNewToReview,
    Empty,
}

impl Due {
    pub fn should_run(self) -> bool {
        matches!(self, Due::Turns | Due::ContextPressure)
    }
}

/// Decide whether an idle session is worth reviewing. Pure, so the policy is
/// testable without a clock or a filesystem.
///
/// Two conditions, both required: the conversation has actually stopped (so a
/// review never interrupts someone mid-thought), and there is enough new material
/// to be worth a model call.
pub fn due(
    session: &crate::core::sessions::Session,
    est_tokens: usize,
    budget_tokens: usize,
    idle_minutes: u32,
    min_turns: u32,
    now: f64,
) -> Due {
    if session.messages.is_empty() {
        return Due::Empty;
    }
    let idle_for = now - session.updated_at;
    if idle_for < idle_minutes as f64 * 60.0 {
        return Due::NotIdleYet;
    }
    if session.turns_since_evolution() >= min_turns as usize {
        return Due::Turns;
    }
    if budget_tokens > 0 && est_tokens as f64 / budget_tokens as f64 > CONTEXT_PRESSURE {
        return Due::ContextPressure;
    }
    Due::NothingNewToReview
}
