use super::*;

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start one assistant's proxy on a free loopback port.
///
/// `allow_loopback` is read per connection, so the Settings switch applies to the next
/// request rather than the next browser.
pub async fn start(persona: &str, data_dir: PathBuf, allow_loopback: Arc<AtomicBool>) -> std::io::Result<Proxy> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let refusals = Arc::new(Mutex::new(VecDeque::new()));
    let purpose = format!("browser · {persona}");
    let persona = persona.to_string();
    let (stop2, refusals2) = (stop.clone(), refusals.clone());
    let task = tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else { continue };
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let ctx = Conn {
                purpose: purpose.clone(),
                persona: persona.clone(),
                data_dir: data_dir.clone(),
                allow_loopback: allow_loopback.load(Ordering::SeqCst),
                refusals: refusals2.clone(),
            };
            tokio::spawn(async move {
                let _ = serve(sock, ctx).await;
            });
        }
    });
    Ok(Proxy { port, stop, refusals, task })
}

pub(super) struct Conn {
    pub(super) purpose: String,
    pub(super) persona: String,
    pub(super) data_dir: PathBuf,
    pub(super) allow_loopback: bool,
    pub(super) refusals: Arc<Mutex<VecDeque<Refusal>>>,
}

impl Conn {
    pub(super) fn refuse(&self, host: &str, reason: &str) {
        let first_in_a_while = {
            let Ok(mut r) = self.refusals.lock() else { return };
            let recent = r.iter().any(|x| x.host == host && now() - x.at < 60.0);
            if r.len() >= REFUSALS_KEPT {
                r.pop_front();
            }
            r.push_back(Refusal { host: host.to_string(), reason: reason.to_string(), at: now() });
            !recent
        };
        // One trail row per host per minute: a page that asks a blocked host for forty
        // images is one refusal worth reading, not forty.
        if first_in_a_while {
            let mut e = crate::core::audit::Entry::event("browser.blocked");
            e.actor = self.persona.clone();
            e.persona = self.persona.clone();
            e.target = host.to_string();
            e.detail = reason.to_string();
            e.ok = false;
            crate::core::audit::append(&self.data_dir, e);
        }
    }
}

pub(super) async fn read_head(sock: &mut TcpStream) -> std::io::Result<(Head, Vec<u8>)> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    loop {
        let n = tokio::time::timeout(HEAD_TIMEOUT, sock.read(&mut chunk))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "no request head"))??;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "closed before a head"));
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(head) = parse_head(&buf) {
            let rest = buf[head.len..].to_vec();
            return Ok((head, rest));
        }
        if buf.len() > HEAD_LIMIT {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "request head too large"));
        }
    }
}

pub(super) async fn deny(sock: &mut TcpStream, reason: &str) -> std::io::Result<()> {
    let body = format!("Refused by Superuser: {reason}.\n");
    let resp = format!(
        "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(resp.as_bytes()).await?;
    sock.shutdown().await
}

pub(super) async fn serve(mut sock: TcpStream, ctx: Conn) -> std::io::Result<()> {
    let (head, rest) = read_head(&mut sock).await?;
    let target = match target_of(&head) {
        Ok(t) => t,
        Err(reason) => return deny(&mut sock, &reason).await,
    };
    let staging = crate::core::repro::is_private_staging(&ctx.persona, &target.host, target.port);
    let expect_loopback = match decide_name_or_staging(&target.host, ctx.allow_loopback, staging) {
        Ok(l) => l,
        Err(reason) => {
            ctx.refuse(&target.host, &reason);
            return deny(&mut sock, &reason).await;
        }
    };
    // Local-only and the Network page. Loopback never left the machine, so it is neither
    // refused by local-only nor recorded — the same rule `egress` applies to itself.
    if !expect_loopback {
        let proto = if target.tunnel { "https" } else { "http" };
        if let Err(reason) = crate::core::egress::allow(&ctx.purpose, &target.host, proto, target.port) {
            ctx.refuse(&target.host, &reason);
            return deny(&mut sock, &reason).await;
        }
    }
    let addrs: Vec<SocketAddr> = match tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::lookup_host((target.host.as_str(), target.port))).await {
        Ok(Ok(a)) => a.collect(),
        _ => return deny(&mut sock, &format!("{} could not be found", target.host)).await,
    };
    let mut chosen = None;
    for a in &addrs {
        match decide_addr_or_staging(a.ip(), expect_loopback, staging) {
            Ok(()) => {
                chosen = Some(*a);
                break;
            }
            Err(reason) => {
                // Any blocked answer refuses the name outright: a host with one public and
                // one private address is a host that can be made to use the private one.
                ctx.refuse(&target.host, &reason);
                return deny(&mut sock, &reason).await;
            }
        }
    }
    let Some(addr) = chosen else {
        return deny(&mut sock, &format!("{} could not be found", target.host)).await;
    };
    // The address that was checked, and no other.
    let mut upstream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(s)) => s,
        _ => return deny(&mut sock, &format!("{} did not answer", target.host)).await,
    };
    if target.tunnel {
        sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
        if !rest.is_empty() {
            upstream.write_all(&rest).await?;
        }
    } else {
        upstream.write_all(&rewrite_for_origin(&head, &target)).await?;
        if !rest.is_empty() {
            upstream.write_all(&rest).await?;
        }
    }
    let _ = tokio::io::copy_bidirectional(&mut sock, &mut upstream).await;
    Ok(())
}
