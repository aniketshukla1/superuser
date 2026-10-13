//! Optional JS rendering for `web_fetch`, as a prototype.
//!
//! `web_fetch` does a plain HTTP GET, so a page that builds itself in the browser returns an
//! empty shell and the agent reads nothing. A headless renderer runs the page's JavaScript
//! and hands back the DOM the user would actually see. Lightpanda is the interesting
//! candidate — it executes JS but has no layout or paint engine, which is exactly the trade
//! we want here: we need the text, never the pixels.
//!
//! OFF BY DEFAULT, and the reason is not caution for its own sake.
//!
//! `web_fetch` today is guarded twice: `webguard::is_blocked_url` refuses private, loopback
//! and link-local addresses, and `egress::client` re-checks EVERY redirect hop, so a public
//! URL cannot bounce onto this machine's network. A separate browser process does its own
//! networking and neither guard can see it.
//!
//! So the renderer never receives the URL the caller asked for. The guarded client fetches
//! first; only its FINAL url — the one that survived the redirect policy — is handed over.
//! The renderer then makes one request to an address the guards have already vetted.
//!
//! One hole remains and it is inherent to running a browser at all: the page's own
//! subresources (scripts, XHR, images) are fetched by the renderer, and the egress guard
//! never sees them. A page that passes every check can still ask the renderer to load
//! something from 169.254.169.254. That cannot be closed from out here — it needs the
//! renderer behind a proxy the guard controls — and it is the reason this ships off, and
//! the reason `local_only` refuses it outright.

use std::process::Stdio;
use std::time::Duration;

/// How long a render may take before we give up and use the plain body instead. A renderer
/// that hangs must cost a few seconds, not the turn.
const TIMEOUT: Duration = Duration::from_secs(20);

/// The command, as configured. Empty (the default) means the feature is off.
///
/// A command rather than a hard-coded `lightpanda` invocation: this is a prototype meant to
/// answer "does rendering actually help?", and pinning one tool's CLI would make swapping in
/// another a code change. The URL is appended as the final argument.
///
/// For Lightpanda that is `lightpanda fetch --dump`.
pub fn command_from(setting: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = setting.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() { None } else { Some(parts) }
}

/// Render `url` and return the resulting DOM, or None if anything at all goes wrong.
///
/// Never returns an error to the caller: this is an enhancement over a fetch that already
/// succeeded, so every failure path is "use what we already have". A renderer that is not
/// installed, crashes, times out or prints nothing must not turn a working `web_fetch` into
/// a failing one.
pub async fn render(setting: &str, url: &str) -> Option<String> {
    let mut argv = command_from(setting)?;
    let program = argv.remove(0);
    argv.push(url.to_string());

    let child = tokio::process::Command::new(&program)
        .args(&argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true) // a timeout must actually end the process, not orphan it
        .spawn()
        .ok()?;

    let out = tokio::time::timeout(TIMEOUT, child.wait_with_output()).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // A renderer that returns less than the plain fetch did is not an improvement; the
    // caller compares and keeps the better one.
    if body.is_empty() { None } else { Some(body) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_command_means_the_feature_is_off() {
        assert!(command_from("").is_none());
        assert!(command_from("   ").is_none());
        assert_eq!(
            command_from("lightpanda fetch --dump").unwrap(),
            vec!["lightpanda", "fetch", "--dump"]
        );
    }

    /// Every failure is "use the plain body". A renderer that is not installed must not turn
    /// a web_fetch that already worked into one that fails.
    #[tokio::test]
    async fn nothing_installed_yields_nothing_rather_than_failing() {
        assert!(render("", "https://example.com").await.is_none());
        assert!(
            render("definitely-not-a-real-binary-9f3a", "https://example.com").await.is_none(),
            "a missing binary must be silent, not fatal"
        );
    }

    #[tokio::test]
    async fn a_renderer_that_prints_nothing_is_not_an_improvement() {
        // `true` exits 0 and prints nothing — the shape of a renderer that ran but found
        // nothing to say.
        assert!(render("true", "https://example.com").await.is_none());
    }

    /// Output from a FAILED render is not output. A renderer that dies half way through a
    /// page has printed something, and taking it would hand the model a truncated document
    /// while reporting success.
    ///
    /// A script rather than `false`, deliberately: `false` prints nothing, so it is caught
    /// by the empty check above whether the exit status is inspected or not — the first
    /// version of this test passed with the status check deleted.
    #[tokio::test]
    async fn output_from_a_failed_render_is_discarded() {
        let dir = std::env::temp_dir().join("su-webrender-test");
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("half-a-page.sh");
        std::fs::write(&script, "#!/bin/sh\necho '<html>half a p'\nexit 3\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let out = render(script.to_str().unwrap(), "https://example.com").await;
        assert!(out.is_none(), "kept output from a renderer that exited 3: {out:?}");
        let _ = std::fs::remove_file(&script);
    }

    /// The rule web_fetch applies around this module: a render is kept only when it
    /// returned MORE than the plain fetch did. That is the entire claim being tested by
    /// this prototype — "JS rendering gets you the content a plain GET misses" — and a
    /// renderer that hands back a shorter shell has disproved it for that page.
    ///
    /// Stated here as the arithmetic rather than driven through web_fetch, which needs a
    /// live network, a tool context and a session on disk.
    #[test]
    fn a_render_is_kept_only_when_it_beat_the_plain_fetch() {
        let keep = |plain: &str, rendered: Option<&str>| -> String {
            match rendered {
                Some(r) if r.len() > plain.len() => r.to_string(),
                _ => plain.to_string(),
            }
        };
        let shell = "<html><div id=app></div></html>";
        let full = "<html><div id=app><h1>The actual article</h1></div></html>";
        assert_eq!(keep(shell, Some(full)), full, "a fuller render must win");
        assert_eq!(keep(full, Some(shell)), full, "a thinner render must not");
        assert_eq!(keep(shell, None), shell, "no renderer means the plain body");
        assert_eq!(keep(shell, Some(shell)), shell, "equal is not better");
    }

    #[tokio::test]
    async fn the_url_is_the_last_argument() {
        // `echo` stands in for the renderer: it prints its argv, so this asserts the URL
        // actually reaches the command, in the position the contract promises.
        let out = render("echo rendered", "https://example.com/page").await.unwrap();
        assert_eq!(out, "rendered https://example.com/page");
    }
}
