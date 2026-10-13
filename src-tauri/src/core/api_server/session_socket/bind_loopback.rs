use super::super::*;

/// Start the API on 127.0.0.1. Returns the bound port and the session token.
/// Bind loopback on `preferred`, or on an OS-assigned port if that one is taken.
///
/// The port used to be a hard-coded 8765 with a single attempt, and losing it took the
/// whole app down: the window opened, the API never started, and every request from the
/// GUI failed with no explanation on screen. Anything at all on that port did it — most
/// often the app's own dev build left running, which is not an exotic situation.
///
/// Falling back is safe because nothing depends on the NUMBER. The window is handed the
/// port it should talk to (see start_local_api in lib.rs), so 8765 is a convenience for
/// humans and tooling, not a contract. Returns whether it had to fall back so the caller
/// can say so out loud rather than quietly serving somewhere unexpected.
///
/// **Both loopbacks, one port.** `localhost` resolves to `::1` before `127.0.0.1` on macOS and
/// on many Linux systems, so a server that holds only the IPv4 side of its port leaves the
/// IPv6 side for anybody. Something listening on `[::1]:8765` would then receive every request
/// a `localhost` client meant for this app — token included. So a port counts as ours only when
/// both sides are: the IPv6 side is bound too and served, and a port whose IPv6 side is held by
/// someone else is treated as taken. A machine with no IPv6 loopback has nothing there to hold.
pub(in crate::core::api_server) async fn bind_loopback(preferred: u16) -> Result<(Loopback, bool), String> {
    let first = match bind_both(preferred).await {
        Ok(l) => return Ok((l, false)),
        Err(e) => e,
    };
    // The OS picks the IPv4 port, and the IPv6 side of that number may be someone else's, so
    // a few tries rather than one.
    let mut last = String::new();
    for _ in 0..8 {
        match bind_both(0).await {
            Ok(l) => return Ok((l, true)),
            Err(e) => last = e,
        }
    }
    Err(format!(
        "Couldn't bind 127.0.0.1:{preferred} ({first}), and no other loopback port was free on both 127.0.0.1 and ::1 either ({last})"
    ))
}

/// The two listeners a port is served on.
pub(crate) struct Loopback {
    pub(in crate::core::api_server) v4: tokio::net::TcpListener,
    /// `None` only on a machine with no IPv6 loopback at all.
    pub(in crate::core::api_server) v6: Option<tokio::net::TcpListener>,
}

impl Loopback {
    pub(in crate::core::api_server) fn port(&self) -> u16 {
        self.v4.local_addr().map(|a| a.port()).unwrap_or(0)
    }
}

pub(in crate::core::api_server) async fn bind_both(port: u16) -> Result<Loopback, String> {
    let v4 = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .await
        .map_err(|e| format!("127.0.0.1:{port}: {e}"))?;
    let got = v4.local_addr().map_err(|e| e.to_string())?.port();
    match tokio::net::TcpListener::bind(SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, got))).await {
        Ok(v6) => Ok(Loopback { v4, v6: Some(v6) }),
        Err(e) if no_ipv6_loopback(&e) => Ok(Loopback { v4, v6: None }),
        Err(e) => Err(format!("[::1]:{got} is held by something else ({e})")),
    }
}

/// This machine has no `::1` to bind — which is not the same as somebody holding it.
pub(in crate::core::api_server) fn no_ipv6_loopback(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::AddrNotAvailable || e.raw_os_error() == Some(libc::EAFNOSUPPORT)
}
