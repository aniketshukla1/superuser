use super::*;

/// The assistant (`personas_builtin/repro.md`).
pub const ASSISTANT: &str = "repro";

pub(super) fn staging_now() -> &'static Mutex<String> {
    static STAGING: OnceLock<Mutex<String>> = OnceLock::new();
    STAGING.get_or_init(|| Mutex::new(String::new()))
}

/// Settings' staging address, kept where the browser's proxy reads it on every connection
/// (`Settings::push_browser_switches`), so a change applies to the next request.
pub fn set_staging(url: &str) {
    if let Ok(mut s) = staging_now().lock() {
        *s = url.trim().to_string();
    }
}

/// Whether a connection from `persona`'s browser to `host:port` is to the staging site —
/// the one site on a private network that Bug Reproduction's browser may reach, since a
/// company's staging usually lives on the company's network. Only this assistant, and only
/// the exact host and port named in Settings; what stays refused regardless (loopback without
/// its switch, cloud metadata, link-local) is `agentbrowser::proxy::decide_name_or_staging`'s.
pub fn is_private_staging(persona: &str, host: &str, port: u16) -> bool {
    persona == ASSISTANT && staging_now().lock().is_ok_and(|s| is_staging_site(&s, host, port))
}

/// `host:port` against a staging address, the way the proxy sees a connection: lowercase,
/// no brackets or trailing dot, and the scheme's own port when the address names none.
pub(super) fn is_staging_site(staging: &str, host: &str, port: u16) -> bool {
    let bare = |h: &str| h.trim_start_matches('[').trim_end_matches(']').trim_end_matches('.').to_ascii_lowercase();
    let staging = staging.trim();
    if staging.is_empty() {
        return false;
    }
    reqwest::Url::parse(&crate::core::browser::normalize_url(staging))
        .ok()
        .and_then(|u| Some((bare(u.host_str()?), u.port_or_known_default()?)))
        .is_some_and(|(h, p)| h == bare(host) && p == port)
}

/// Whether `url` is on the staging address. Host and port both: staging on `:3000` is not the
/// service on `:8080` (the same rule as the browser's site grants).
pub fn on_staging(url: &str, staging: &str) -> Result<(), String> {
    let staging = staging.trim();
    if staging.is_empty() {
        return Err("no staging address is set, so there is nowhere to reproduce a bug. The person sets it in \
                    Settings ▸ General ▸ Bug Reproduction — reproduction happens on staging only."
            .into());
    }
    let want = crate::core::agent_exec::site_of(&crate::core::browser::normalize_url(staging));
    let got = crate::core::agent_exec::site_of(&crate::core::browser::normalize_url(url));
    if !want.is_empty() && got == want {
        Ok(())
    } else {
        Err(format!(
            "`{}` isn't staging ({want}). Bug Reproduction works on staging only — never production, where customers' data is. \
             If the report is about another environment, say so and stop.",
            if got.is_empty() { url } else { &got }
        ))
    }
}

/// Whether one of the assistant's browser calls may go ahead. `page` is the address
/// `browser_navigate` opens, or for every other call the page already open (`None` before there
/// is one). Reading counts as much as acting: a production page read is production data seen.
pub fn may_use(tool: &str, page: Option<&str>, staging: &str) -> Result<(), String> {
    match (tool, page) {
        ("browser_navigate", p) => on_staging(p.unwrap_or(""), staging),
        _ if staging.trim().is_empty() => on_staging("", staging),
        // A person's hands: how a sign-in on another site (single sign-on) gets done. The
        // person does it and the assistant sees none of it.
        ("browser_request_help", _) => Ok(()),
        // Nothing open yet, or a blank tab: nothing to read and nothing to act on.
        (_, None | Some("") | Some("about:blank")) => Ok(()),
        (_, Some(url)) => on_staging(url, staging),
    }
}

/// Where a staging address landed. A redirect off staging — to production, or to a sign-in
/// on another site — keeps that page from the assistant.
pub fn landed(url: &str, staging: &str) -> Result<(), String> {
    on_staging(url, staging).map_err(|_| {
        let site = crate::core::agent_exec::site_of(url);
        format!(
            "that landed on `{}`, which isn't staging, so its page is kept from you. If it's a sign-in on another site \
             (single sign-on), use browser_request_help so the person signs in themselves; otherwise stop and say where it went.",
            if site.is_empty() { url } else { &site }
        )
    })
}

/// A URL's scheme and host checked for the Settings field: an http(s) address with a host.
/// Said to a person, in Settings.
pub fn staging_problem(url: &str) -> Option<String> {
    let u = url.trim();
    if u.is_empty() {
        return None;
    }
    // A scheme is checked as typed: normalizing would read `ftp://host` as a host to put
    // https in front of.
    if u.contains("://") && !(u.starts_with("http://") || u.starts_with("https://")) {
        return Some(format!("“{u}” isn't a web address — use something like https://staging.example.com."));
    }
    let host = crate::core::agent_exec::site_of(&crate::core::browser::normalize_url(u));
    if host.is_empty() {
        return Some(format!("“{u}” isn't a web address — use something like https://staging.example.com."));
    }
    // What the browser never reaches, staging or not: named here rather than at the first
    // refused page.
    let bare = crate::core::boundaries::host_of(&crate::core::browser::normalize_url(u));
    let bare = bare.trim_start_matches('[').trim_end_matches(']');
    let reserved = bare.parse::<std::net::IpAddr>().is_ok_and(|ip| {
        !ip.is_loopback() && !crate::core::webguard::is_private_network(ip) && crate::core::webguard::is_blocked_host(bare)
    });
    if crate::core::webguard::is_metadata_name(bare) || reserved {
        return Some(format!(
            "“{host}” is a cloud-metadata, link-local or reserved address, which the assistant's browser never reaches."
        ));
    }
    None
}
