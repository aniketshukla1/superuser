use super::*;

/// The viewport every page is laid out at. A user's clicks on the live view are mapped
/// back into these coordinates, so it is fixed rather than whatever a window happens to be.
pub const VIEWPORT: (u32, u32) = (1280, 800);

/// The first Chrome-family browser found, or `None`. `SUPERUSER_CHROME` wins when set, so
/// a user with a browser somewhere unusual — or a test with Chrome for Testing — can say so.
pub fn find() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SUPERUSER_CHROME").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    #[cfg(target_os = "macos")]
    {
        const APPS: &[&str] = &["Google Chrome", "Google Chrome for Testing", "Chromium", "Brave Browser", "Microsoft Edge"];
        let mut roots = vec![PathBuf::from("/Applications")];
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Applications"));
        }
        for root in &roots {
            for app in APPS {
                let p = root.join(format!("{app}.app/Contents/MacOS/{app}"));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        const NAMES: &[&str] = &["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "brave-browser", "microsoft-edge"];
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                for name in NAMES {
                    let p = dir.join(name);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

/// The command line an assistant's browser starts with.
pub fn args(profile: &Path, proxy_port: u16, headless: bool) -> Vec<String> {
    let mut a: Vec<String> = vec![
        format!("--user-data-dir={}", profile.display()),
        // The protocol over fds 3 and 4, not a port. See `cdp`.
        "--remote-debugging-pipe".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        // Chrome's own traffic — update checks, safe browsing lists, sync, metrics — is
        // not the assistant's work, and every byte of it would otherwise arrive at the
        // proxy looking like it was.
        "--disable-background-networking".into(),
        "--disable-component-update".into(),
        "--disable-sync".into(),
        "--disable-default-apps".into(),
        "--disable-client-side-phishing-detection".into(),
        "--disable-domain-reliability".into(),
        "--metrics-recording-only".into(),
        "--no-pings".into(),
        "--disable-features=Translate,OptimizationHints,MediaRouter,DialMediaRouteProvider,AutofillServerCommunication".into(),
        // Without this, sites that check `navigator.webdriver` refuse a person signing
        // in during a takeover — Google's sign-in does. The governed path
        // is unchanged either way; this only stops the browser announcing itself.
        "--disable-blink-features=AutomationControlled".into(),
        format!("--window-size={},{}", VIEWPORT.0, VIEWPORT.1),
        // Every connection through this app's proxy, loopback included (`<-loopback>`
        // removes Chrome's built-in exemption), and nothing that can route around it:
        // QUIC is UDP, and WebRTC would otherwise open UDP sockets of its own.
        format!("--proxy-server=http://127.0.0.1:{proxy_port}"),
        "--proxy-bypass-list=<-loopback>".into(),
        "--disable-quic".into(),
        "--force-webrtc-ip-handling-policy=disable_non_proxied_udp".into(),
    ];
    #[cfg(target_os = "macos")]
    a.push("--use-mock-keychain".into());
    if headless {
        a.push("--headless=new".into());
    }
    a.push("about:blank".into());
    a
}

/// A browser that is running, and the pipes to it.
#[cfg(unix)]
pub struct Launched {
    pub child: tokio::process::Child,
    pub pid: u32,
    pub to_chrome: tokio::net::unix::pipe::Sender,
    pub from_chrome: tokio::net::unix::pipe::Receiver,
}

#[cfg(unix)]
pub(super) fn pipe_pair() -> std::io::Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    use std::os::fd::FromRawFd;
    let mut fds = [0i32; 2];
    // SAFETY: `pipe` writes two descriptors into the array it is given.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    for fd in fds {
        // Close-on-exec on both of ours; the child gets its own copies from `dup2`,
        // which never carry the flag.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    // SAFETY: both descriptors were just created and are owned by nobody else.
    Ok(unsafe { (std::os::fd::OwnedFd::from_raw_fd(fds[0]), std::os::fd::OwnedFd::from_raw_fd(fds[1])) })
}

/// Start the browser with its protocol on fds 3 (it reads) and 4 (it writes), in its own
/// process group so stopping it stops its renderers and helpers with it. Chrome's own
/// output goes to `log`, overwritten per launch: when a start fails, that file says why.
#[cfg(unix)]
pub fn launch(binary: &Path, args: &[String], log: &Path) -> std::io::Result<Launched> {
    use std::os::fd::AsRawFd;
    let (chrome_reads, we_write) = pipe_pair()?;
    let (we_read, chrome_writes) = pipe_pair()?;
    let (child_in, child_out) = (chrome_reads.as_raw_fd(), chrome_writes.as_raw_fd());

    let log_file = std::fs::File::create(log)?;
    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::from(log_file))
        .kill_on_drop(true);
    // SAFETY: only async-signal-safe calls between fork and exec.
    unsafe {
        cmd.pre_exec(move || {
            // Moved above 10 first, so `dup2` onto 3 and 4 cannot clobber a pipe end
            // that happened to be numbered 3 or 4 already.
            let a = libc::fcntl(child_in, libc::F_DUPFD, 10);
            let b = libc::fcntl(child_out, libc::F_DUPFD, 10);
            if a < 0 || b < 0 || libc::dup2(a, 3) < 0 || libc::dup2(b, 4) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            libc::close(a);
            libc::close(b);
            libc::setpgid(0, 0);
            Ok(())
        });
    }
    let child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);
    // The child's ends, closed on our side now that it has its copies. Holding them would
    // keep our reader from ever seeing Chrome hang up.
    drop(chrome_reads);
    drop(chrome_writes);
    Ok(Launched {
        child,
        pid,
        to_chrome: tokio::net::unix::pipe::Sender::from_owned_fd(we_write)?,
        from_chrome: tokio::net::unix::pipe::Receiver::from_owned_fd(we_read)?,
    })
}

/// Kill a browser's whole process group.
#[cfg(unix)]
pub fn kill_group(pid: u32) {
    if pid > 1 {
        // SAFETY: a signal to a process group this app created.
        unsafe { libc::killpg(pid as i32, libc::SIGKILL) };
    }
}

#[cfg(not(unix))]
pub fn kill_group(_pid: u32) {}
