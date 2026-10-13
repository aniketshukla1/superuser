//! Per-session freeze

use super::*;

/// The rendered block a session started with, so it stays identical for the whole
/// conversation.
///
/// The profile changes underneath a running conversation — the learner writes to
/// it while the user is still typing. Re-rendering per turn would rewrite the
/// system prompt mid-conversation and invalidate the provider's prompt cache on
/// every single turn, which costs real money and latency to reflect a preference
/// that could perfectly well wait. Edits land in the next session.
///
/// Process-lifetime only: a restart re-renders, which is exactly right, since the
/// prompt cache didn't survive either.
pub(super) fn freezer() -> &'static std::sync::Mutex<Vec<(String, String)>> {
    static FREEZER: std::sync::OnceLock<std::sync::Mutex<Vec<(String, String)>>> = std::sync::OnceLock::new();
    FREEZER.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// How many sessions' frozen blocks to keep. A bound, not a tuning knob: the
/// oldest entry being re-rendered costs one file read.
pub(super) const FROZEN_SESSIONS: usize = 64;

/// The profile block for a session, rendered once and then held still.
pub fn frozen_block(data_dir: &Path, session_id: &str) -> String {
    if let Ok(f) = freezer().lock() {
        if let Some((_, block)) = f.iter().find(|(id, _)| id == session_id) {
            return block.clone();
        }
    }
    let block = Learned::load(data_dir).system_block();
    if let Ok(mut f) = freezer().lock() {
        f.retain(|(id, _)| id != session_id);
        f.push((session_id.to_string(), block.clone()));
        if f.len() > FROZEN_SESSIONS {
            let excess = f.len() - FROZEN_SESSIONS;
            f.drain(0..excess);
        }
    }
    block
}

/// Forget a session's frozen block — used when the profile is edited by hand, so
/// the user doesn't have to start a new conversation to see their own edit apply.
pub fn thaw_all() {
    if let Ok(mut f) = freezer().lock() {
        f.clear();
    }
}
