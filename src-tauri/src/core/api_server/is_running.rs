use super::*;

pub(crate) fn is_running(id: &str) -> bool {
    running_set().lock().map(|s| s.contains_key(id)).unwrap_or(false)
}
