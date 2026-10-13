use super::*;

pub(super) const DOW: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub(super) fn gen_id(prefix: &str) -> String {
    let n: u64 = rand::random();
    format!("{prefix}-{:010x}", n & 0xff_ffff_ffff)
}

pub(super) fn tru() -> bool {
    true
}

pub(super) fn default_agent() -> String {
    crate::core::personas::DEFAULT_PERSONA_ID.to_string()
}

pub(super) fn local_tz() -> String {
    "local".to_string()
}

pub(super) fn status_running() -> String {
    "running".to_string()
}

pub(super) fn trigger_schedule() -> String {
    "schedule".to_string()
}
