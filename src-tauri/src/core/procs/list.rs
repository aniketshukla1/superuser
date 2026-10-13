use super::*;

/// This session's processes: handle, command, and exit code if it has exited.
pub fn list(session: &str) -> Vec<(String, String, Option<i32>)> {
    let mut t = lock();
    let mut out: Vec<(String, String, Option<i32>)> = t
        .iter_mut()
        .filter(|(_, p)| p.session == session)
        .map(|(h, p)| {
            reap(p);
            (h.clone(), p.command.clone(), p.out.lock().ok().and_then(|o| o.exit))
        })
        .collect();
    out.sort();
    out
}

/// Nothing the agent started should outlive the app holding a port hostage.
pub fn stop_all() {
    let mut t = lock();
    for (_, mut p) in t.drain() {
        let _ = p.child.kill();
    }
}

/// The process behind `handle`, if `session` owns it. A handle from another
/// conversation is reported as unknown rather than as forbidden: saying it exists
/// would be telling one session about another's work.
pub(super) fn owned<'a>(t: &'a mut HashMap<String, Proc>, session: &str, handle: &str) -> Result<&'a mut Proc, String> {
    match t.get_mut(handle) {
        Some(p) if p.session == session => Ok(p),
        _ => Err(format!("error: no process {handle:?} in this conversation — process_start returns the handle")),
    }
}
