use super::super::*;

/// Did this request stay on the machine? Reuses the SSRF host table, because
/// "private enough that an agent must not reach it" and "private enough that it
/// never left" are the same set of addresses.
pub(in crate::core::egress) fn is_local(host: &str) -> bool {
    crate::core::webguard::is_blocked_host(host)
}

/// Ask permission for egress that isn't HTTP, and put it on the record.
///
/// The client below intercepts every request because it owns the builder. SMTP
/// and IMAP have no builder to own — `lettre` and `imap` open their own sockets —
/// so the email connector has to declare itself. That is a weaker guarantee than
/// the HTTP path and it would be dishonest to pretend otherwise; what keeps it
/// true is that the lint fails if a file speaks either protocol without calling
/// this first.
///
/// `port` rides along in the path slot, since "which port" is the closest thing a
/// socket has to a path and it is what tells submission from IMAPS at a glance.
pub fn allow(purpose: &str, host: &str, protocol: &str, port: u16) -> Result<(), String> {
    if is_local(host) {
        return Ok(());
    }
    if local_only() {
        let error = format!(
            "Local-only mode is on, so nothing may leave this machine — {host} was not contacted. \
             Turn it off in Settings to allow this."
        );
        record(Egress {
            at: now(),
            purpose: purpose.into(),
            method: protocol.to_uppercase(),
            host: host.into(),
            path: format!(":{port}"),
            sent: 0,
            status: None,
            error: Some(error.clone()),
            blocked: true,
        });
        return Err(error);
    }
    record(Egress {
        at: now(),
        purpose: purpose.into(),
        method: protocol.to_uppercase(),
        host: host.into(),
        path: format!(":{port}"),
        sent: 0,
        status: None,
        error: None,
        blocked: false,
    });
    Ok(())
}
