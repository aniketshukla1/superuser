use super::*;

pub(super) const HEAD_LIMIT: usize = 16 * 1024;

pub(super) const HEAD_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) const REFUSALS_KEPT: usize = 32;

/// A request head, as much of it as the proxy reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    pub method: String,
    pub target: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    /// Bytes the head took, `\r\n\r\n` included.
    pub len: usize,
}

/// Where a request is going.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    /// `CONNECT` — a tunnel, bytes passed through untouched.
    pub tunnel: bool,
    /// For plain HTTP, the path and query to send upstream.
    pub path: String,
}

/// One connection the proxy would not make.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub host: String,
    pub reason: String,
    pub at: f64,
}

pub fn parse_head(buf: &[u8]) -> Option<Head> {
    let end = buf.windows(4).position(|w| w == b"\r\n\r\n")?;
    let text = std::str::from_utf8(&buf[..end]).ok()?;
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();
    let version = first.next()?.to_string();
    if first.next().is_some() || !version.starts_with("HTTP/") {
        return None;
    }
    let headers = lines
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();
    Some(Head { method, target, version, headers, len: end + 4 })
}

pub(super) fn split_host_port(authority: &str, default_port: u16) -> Option<(String, u16)> {
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None if after.is_empty() => default_port,
            None => return None,
        };
        return Some((host.to_ascii_lowercase(), port));
    }
    match authority.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => Some((h.to_ascii_lowercase(), p.parse().ok()?)),
        Some(_) => None, // an unbracketed IPv6 literal is ambiguous
        None => Some((authority.to_ascii_lowercase(), default_port)),
    }
}

/// Where the request is going, from its head. `Err` is the reason it can't be served.
pub fn target_of(head: &Head) -> Result<Target, String> {
    if head.method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_host_port(&head.target, 443).ok_or("that CONNECT target isn't host:port")?;
        return Ok(Target { host: host.trim_end_matches('.').to_string(), port, tunnel: true, path: String::new() });
    }
    let (scheme, rest) = head.target.split_once("://").ok_or("this proxy only takes absolute URLs")?;
    if !scheme.eq_ignore_ascii_case("http") {
        // HTTPS arrives as CONNECT; anything else — ftp, ws without upgrade — is not served.
        return Err(format!("{scheme} isn't something the browser may reach through this proxy"));
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = split_host_port(authority, 80).ok_or("that URL has no usable host")?;
    Ok(Target { host: host.trim_end_matches('.').to_string(), port, tunnel: false, path: path.to_string() })
}

pub(crate) fn is_loopback_name(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    h == "localhost" || h.ends_with(".localhost") || h.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// The name-level decision, before anything is resolved. `Ok(true)` when the target is
/// this machine (and allowed), so the address check knows loopback is expected.
pub fn decide_name(host: &str, allow_loopback: bool) -> Result<bool, String> {
    if host.is_empty() {
        return Err("no host".into());
    }
    if is_loopback_name(host) {
        return if allow_loopback {
            Ok(true)
        } else {
            Err(format!(
                "{host} is this machine, and the assistant's browser doesn't reach this machine's own services unless you allow it in Settings ▸ Browser"
            ))
        };
    }
    if crate::core::webguard::is_blocked_host(host) {
        return Err(format!("{host} is a private or cloud-metadata address, which the assistant's browser never reaches"));
    }
    Ok(false)
}

/// `decide_name`, and its one exception: `staging` is true for the site the person named as
/// Bug Reproduction's staging (`core::repro::is_private_staging`), which may be on a private
/// network — a company's staging usually is. Loopback keeps its own switch, and a
/// cloud-metadata name or a link-local or reserved address stays refused whatever is named.
pub fn decide_name_or_staging(host: &str, allow_loopback: bool, staging: bool) -> Result<bool, String> {
    match decide_name(host, allow_loopback) {
        Err(_) if staging && !is_loopback_name(host) && staging_may_be(host) => Ok(false),
        decided => decided,
    }
}

/// `decide_addr`, and the same exception: the staging site may resolve to a private network.
pub fn decide_addr_or_staging(ip: IpAddr, expect_loopback: bool, staging: bool) -> Result<(), String> {
    match decide_addr(ip, expect_loopback) {
        Err(_) if staging && crate::core::webguard::is_private_network(ip) => Ok(()),
        decided => decided,
    }
}

/// A name the staging exception may cover: not a cloud-metadata name, and, when it's an
/// address, one on a private network.
pub(super) fn staging_may_be(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    !crate::core::webguard::is_metadata_name(h)
        && h.parse::<IpAddr>().map_or(true, crate::core::webguard::is_private_network)
}

/// The address-level decision, for each address the name resolved to.
pub fn decide_addr(ip: IpAddr, expect_loopback: bool) -> Result<(), String> {
    if ip.is_loopback() {
        return if expect_loopback { Ok(()) } else { Err(format!("that name resolves to this machine ({ip})")) };
    }
    if crate::core::webguard::is_blocked_host(&ip.to_string()) {
        return Err(format!("that name resolves to a private address ({ip})"));
    }
    Ok(())
}

/// A plain-HTTP head rewritten for the origin: the request line in origin form, the
/// proxy's own headers dropped, and one request per connection.
pub fn rewrite_for_origin(head: &Head, target: &Target) -> Vec<u8> {
    let mut out = format!("{} {} {}\r\n", head.method, target.path, head.version);
    for (k, v) in &head.headers {
        let key = k.to_ascii_lowercase();
        if key.starts_with("proxy-") || key == "connection" || key == "keep-alive" {
            continue;
        }
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("Connection: close\r\n\r\n");
    out.into_bytes()
}

pub(super) fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// A running proxy for one assistant.
pub struct Proxy {
    pub port: u16,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) refusals: Arc<Mutex<VecDeque<Refusal>>>,
    pub(super) task: tokio::task::JoinHandle<()>,
}

impl Proxy {
    /// The most recent refusals, newest last — so a navigation that failed can say which
    /// host was stopped and why, rather than Chrome's `ERR_TUNNEL_CONNECTION_FAILED`.
    pub fn refusals(&self) -> Vec<Refusal> {
        self.refusals.lock().map(|r| r.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.task.abort();
    }
}
