use super::*;

/// Seconds of silence tolerated from a cloud provider.
pub const DEFAULT_CLOUD_SECS: u32 = 60;

/// Seconds of silence tolerated from a model on this machine.
pub const DEFAULT_LOCAL_SECS: u32 = 300;

pub(super) static CLOUD: AtomicU32 = AtomicU32::new(DEFAULT_CLOUD_SECS);

pub(super) static LOCAL: AtomicU32 = AtomicU32::new(DEFAULT_LOCAL_SECS);

/// Set by `Settings::load` and `Settings::save`, the two functions every settings
/// change passes through — the same arrangement `egress::set_local_only` uses, so the
/// model clients need no settings handle to honour the limit.
pub fn set_limits(cloud_secs: u32, local_secs: u32) {
    CLOUD.store(cloud_secs, Ordering::Relaxed);
    LOCAL.store(local_secs, Ordering::Relaxed);
}

/// How much silence to tolerate. `None` means the watchdog is off (a limit of 0).
pub fn limit(local: bool) -> Option<Duration> {
    let secs = if local { LOCAL.load(Ordering::Relaxed) } else { CLOUD.load(Ordering::Relaxed) };
    (secs > 0).then(|| Duration::from_secs(secs as u64))
}

/// How often a wait wakes to check Stop and the clock. Also what makes Stop land
/// during a silence — it used to be checked only when a chunk arrived, so a stalled
/// stream could not be stopped either.
pub const TICK: Duration = Duration::from_millis(250);

/// Why a wait ended without an item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quiet {
    /// Stop was pressed.
    Stopped,
    /// Nothing arrived for this long.
    Stalled(Duration),
}

/// Wait for a future, but no longer than `limit` of silence, and not past Stop.
///
/// The future is polled in `TICK` slices and never dropped between them, so this is
/// safe for anything — including a request that is still being sent.
pub async fn within<F: std::future::Future>(fut: F, limit: Option<Duration>, cancel: &AtomicBool) -> Result<F::Output, Quiet> {
    tokio::pin!(fut);
    let mut silent = Duration::ZERO;
    loop {
        match tokio::time::timeout(TICK, &mut fut).await {
            Ok(v) => return Ok(v),
            Err(_) => {
                silent += TICK;
                if cancel.load(Ordering::SeqCst) {
                    return Err(Quiet::Stopped);
                }
                if let Some(l) = limit {
                    if silent >= l {
                        return Err(Quiet::Stalled(silent));
                    }
                }
            }
        }
    }
}

/// A duration the way a person says it.
pub fn in_words(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=1 => "a second".into(),
        2..=59 => format!("{s} seconds"),
        60..=89 => "a minute".into(),
        90..=119 => "a minute and a half".into(),
        _ if s % 60 == 0 => format!("{} minutes", s / 60),
        _ => format!("{} minutes", (s + 30) / 60),
    }
}

/// What the turn ends with: what
/// happened, for how long, and the two things worth trying.
pub fn sentence(model: &str, silent_for: Duration, local: bool) -> String {
    format!(
        "{model} went quiet: nothing came back for {}, so the turn was stopped. Try again, or {}.",
        in_words(silent_for),
        if local {
            "check that Ollama is still running — a model that is still loading can take this long, and the limit is in Settings ▸ Models"
        } else {
            "check that the provider is reachable"
        }
    )
}

/// End a turn whose model went quiet: the sentence the turn fails with, and the
/// trail's row for it. One function for every provider branch, so the three places
/// a stall can be noticed cannot end a turn three different ways.
pub fn end_turn(data_dir: &std::path::Path, session: &str, persona: &str, model: &str, silent_for: Duration, local: bool) -> String {
    let said = sentence(model, silent_for, local);
    record(data_dir, session, persona, model, silent_for, local);
    said
}

/// The trail's record of it. Written after the sentence is already on its way.
pub fn record(data_dir: &std::path::Path, session: &str, persona: &str, model: &str, silent_for: Duration, local: bool) {
    let mut e = crate::core::audit::Entry::event("turn.stalled");
    e.actor = persona.to_string();
    e.persona = persona.to_string();
    e.session = session.to_string();
    e.initiator = Some(crate::core::audit::Initiator::from_session(session));
    e.target = model.to_string();
    e.ok = false;
    e.detail = format!(
        "{model} sent nothing for {} ({} limit), so the turn was ended",
        in_words(silent_for),
        if local { "local" } else { "cloud" }
    );
    crate::core::audit::append(data_dir, e);
}
