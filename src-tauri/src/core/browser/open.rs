use super::*;

/// The label of the one child webview we manage. Single browser tab in v1.
pub const LABEL: &str = "browser";

/// Turn whatever the user typed into something navigable. Bare hosts get https://,
/// anything with spaces or no dot becomes a search. Mirrors an address bar's habits.
pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "about:blank".into();
    }
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("about:") {
        return s.to_string();
    }
    // A token with a dot and no whitespace reads as a host ("example.com", "localhost:3000").
    let looks_like_host = !s.contains(char::is_whitespace) && (s.contains('.') || s.starts_with("localhost"));
    if looks_like_host {
        return format!("https://{s}");
    }
    format!("https://duckduckgo.com/?q={}", urlencode(s))
}

pub(super) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// True for schemes the browser tab must never load. `file://` would hand a remote
/// page the local filesystem through the same webview; the rest are OS-level escapes.
pub fn is_blocked_scheme(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    ["file:", "javascript:", "data:", "blob:", "chrome:", "tauri:", "asset:"]
        .iter()
        .any(|s| lower.starts_with(s))
}

pub(super) fn bounds<R: Runtime>(
    wv: &tauri::webview::Webview<R>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    wv.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    // A zero or negative dimension makes wry unhappy; clamp to something it accepts.
    wv.set_size(LogicalSize::new(w.max(1.0), h.max(1.0))).map_err(|e| e.to_string())
}

/// Create the child webview if it doesn't exist yet, else navigate the existing one.
/// Idempotent — the React side calls this whenever the Browser tab activates.
pub fn open<R: Runtime>(
    app: &tauri::AppHandle<R>,
    url: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<String, String> {
    let target = normalize_url(url);
    if is_blocked_scheme(&target) {
        return Err(format!("Refusing to open {target} in the browser panel."));
    }

    if let Some(wv) = app.get_webview(LABEL) {
        bounds(&wv, x, y, w, h)?;
        let _ = wv.show();
        if !url.trim().is_empty() {
            let parsed = target.parse().map_err(|_| format!("Not a URL: {target}"))?;
            wv.navigate(parsed).map_err(|e| e.to_string())?;
        }
        return Ok(target);
    }

    let window = app
        .get_window("main")
        .ok_or_else(|| "The main window is not available.".to_string())?;
    let parsed = target.parse().map_err(|_| format!("Not a URL: {target}"))?;

    let handle = app.clone();
    let builder = tauri::webview::WebviewBuilder::new(LABEL, WebviewUrl::External(parsed))
        // Every navigation reports back so the address bar and the back/forward
        // buttons reflect what the page actually did (redirects, in-page links).
        .on_navigation(move |url| {
            if is_blocked_scheme(url.as_str()) {
                return false;
            }
            let _ = handle.emit_to("main", "browser-nav", url.to_string());
            true
        });

    window
        .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(w.max(1.0), h.max(1.0)))
        .map_err(|e| e.to_string())?;
    Ok(target)
}

pub fn navigate<R: Runtime>(app: &tauri::AppHandle<R>, url: &str) -> Result<String, String> {
    let target = normalize_url(url);
    if is_blocked_scheme(&target) {
        return Err(format!("Refusing to open {target} in the browser panel."));
    }
    let wv = app.get_webview(LABEL).ok_or_else(|| "The browser is not open.".to_string())?;
    let parsed = target.parse().map_err(|_| format!("Not a URL: {target}"))?;
    wv.navigate(parsed).map_err(|e| e.to_string())?;
    Ok(target)
}

/// Move/resize to track the panel. Called on every layout change, so a missing
/// webview is not an error — the Browser tab simply isn't open.
pub fn set_bounds<R: Runtime>(app: &tauri::AppHandle<R>, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    match app.get_webview(LABEL) {
        Some(wv) => bounds(&wv, x, y, w, h),
        None => Ok(()),
    }
}

/// Hide without destroying — keeps the page and its scroll position alive while
/// another tab, a modal, or the search palette is on top of where it sits.
pub fn set_visible<R: Runtime>(app: &tauri::AppHandle<R>, visible: bool) -> Result<(), String> {
    let Some(wv) = app.get_webview(LABEL) else { return Ok(()) };
    if visible { wv.show() } else { wv.hide() }.map_err(|e| e.to_string())
}

pub fn close<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// History and reload. Tauri exposes no history API for a child webview, so these
/// run in the page — which is exactly what the buttons mean anyway.
pub fn history<R: Runtime>(app: &tauri::AppHandle<R>, action: &str) -> Result<(), String> {
    let wv = app.get_webview(LABEL).ok_or_else(|| "The browser is not open.".to_string())?;
    let script = match action {
        "back" => "history.back()",
        "forward" => "history.forward()",
        "reload" => "location.reload()",
        other => return Err(format!("Unknown history action: {other}")),
    };
    wv.eval(script).map_err(|e| e.to_string())
}

pub fn current_url<R: Runtime>(app: &tauri::AppHandle<R>) -> Option<String> {
    app.get_webview(LABEL).and_then(|wv| wv.url().ok()).map(|u| u.to_string())
}

/// A dev server the workspace knows how to start.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct DevServer {
    pub name: String,
    pub command: String,
    pub port: Option<u16>,
    pub url: String,
}
