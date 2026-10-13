use super::super::*;

/// Held for as long as someone is watching. Dropping it may stop the cast.
pub struct Watcher {
    pub(in crate::core::agentbrowser::computer) computer: Arc<Computer>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let c = self.computer.clone();
        if c.shared.viewers.fetch_sub(1, Ordering::SeqCst) == 1 {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let guard = c.running.lock().await;
                    if let Ok(cdp) = open_cdp(&guard) {
                        stop_cast(&cdp, &c.shared).await;
                    }
                });
            }
        }
    }
}

pub(in crate::core::agentbrowser::computer) fn open_cdp(guard: &Option<Running>) -> Result<Arc<Cdp>, String> {
    match guard {
        Some(r) if !r.cdp.is_closed() => Ok(r.cdp.clone()),
        _ => Err("Your browser isn't open. Open a page with browser_navigate first.".into()),
    }
}

/// Stop the least recently used browsers until there is room for one more.
/// Called from `ensure`, with the caller's own browser lock held — so it waits for no other
/// browser's lock. It used to ask each one `is_running`, which takes that lock: two
/// assistants starting their browsers at once each waited for the other's, forever. A
/// launched browser has a pid until it's stopped, and that is read without waiting.
pub(in crate::core::agentbrowser::computer) fn make_room(for_persona: &str) {
    let mut live: Vec<Arc<Computer>> =
        all().into_iter().filter(|c| c.persona() != for_persona && c.shared.pid.load(Ordering::SeqCst) != 0).collect();
    if live.len() < MAX_LIVE {
        return;
    }
    live.sort_by(|a, b| a.last_used().partial_cmp(&b.last_used()).unwrap_or(std::cmp::Ordering::Equal));
    for c in live {
        let driving = c.shared.control.lock().map(|k| k.holder() == Holder::User).unwrap_or(false);
        if !driving {
            // In the background, for the same reason: stopping waits for that browser's lock,
            // which its own start may be holding while it makes room too.
            tokio::spawn(async move {
                c.stop("Another assistant needed a browser, and this one was the least recently used.").await;
            });
            return;
        }
    }
}

pub(in crate::core::agentbrowser::computer) fn purge_frames(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let cutoff = std::time::SystemTime::now() - Duration::from_secs_f64(FRAME_KEEP_DAYS * 86_400.0);
    for e in rd.flatten() {
        let old = e.metadata().ok().and_then(|m| m.modified().ok()).is_some_and(|t| t < cutoff);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

pub(in crate::core::agentbrowser::computer) async fn attach(cdp: &Cdp, shared: &Shared, target: &str) -> Result<(), String> {
    let attached = cdp.call(None, "Target.attachToTarget", json!({"targetId": target, "flatten": true}), ACTION_TIMEOUT).await?;
    let session = attached["sessionId"].as_str().unwrap_or_default().to_string();
    for (method, params) in [
        ("Page.enable", json!({})),
        ("DOM.enable", json!({})),
        ("Accessibility.enable", json!({})),
        // What the page says on its own — console errors, uncaught exceptions, failed
        // requests — for `browser_console`. `Log` carries the failed requests without the
        // Network domain's event per request, which a busy page would flood the queue with.
        ("Runtime.enable", json!({})),
        ("Log.enable", json!({})),
        (
            "Emulation.setDeviceMetricsOverride",
            json!({"width": chrome::VIEWPORT.0, "height": chrome::VIEWPORT.1, "deviceScaleFactor": 1, "mobile": false}),
        ),
    ] {
        let _ = cdp.call(Some(&session), method, params, ACTION_TIMEOUT).await;
    }
    let previous = {
        let mut p = shared.page.lock().map_err(|_| "page state unavailable".to_string())?;
        let prev = p.casting.take();
        *p = Page { target: target.to_string(), session: session.clone(), casting: None };
        prev
    };
    if let Ok(mut r) = shared.refs.lock() {
        // A ref names an element on the page it came from: a new page retires them all.
        r.bump();
    }
    if let Some(old) = previous {
        let _ = cdp.call(Some(&old), "Page.stopScreencast", json!({}), Duration::from_secs(2)).await;
    }
    if shared.viewers.load(Ordering::SeqCst) > 0 {
        start_cast(cdp, shared).await;
    }
    Ok(())
}

pub(in crate::core::agentbrowser::computer) async fn start_cast(cdp: &Cdp, shared: &Shared) {
    let session = shared.page.lock().map(|p| p.session.clone()).unwrap_or_default();
    if session.is_empty() {
        return;
    }
    let started = cdp
        .call(
            Some(&session),
            "Page.startScreencast",
            json!({"format": "jpeg", "quality": 70, "maxWidth": chrome::VIEWPORT.0, "maxHeight": chrome::VIEWPORT.1, "everyNthFrame": 1}),
            ACTION_TIMEOUT,
        )
        .await;
    if started.is_ok() {
        if let Ok(mut p) = shared.page.lock() {
            p.casting = Some(session);
        }
    }
}

/// One event as a line for a bug report, or `None` for the chatter: console errors and
/// warnings (not logs), uncaught exceptions, and network or security problems Chrome reported.
/// Console messages come from `Runtime` only, so none is counted twice. The text is the page's.
pub fn note_of(method: &str, p: &Value) -> Option<String> {
    let cut = |s: &str| -> String {
        s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ").chars().take(300).collect()
    };
    match method {
        "Runtime.consoleAPICalled" => {
            let label = match p["type"].as_str()? {
                "error" | "assert" => "console error",
                "warning" => "console warning",
                _ => return None,
            };
            let text = p["args"]
                .as_array()
                .map(|args| {
                    args.iter()
                        .map(|a| match &a["value"] {
                            Value::String(s) => s.clone(),
                            Value::Null => a["description"].as_str().unwrap_or("").to_string(),
                            other => other.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            Some(format!("{label}: {}", cut(&text)))
        }
        "Runtime.exceptionThrown" => {
            let d = &p["exceptionDetails"];
            let what = d["exception"]["description"].as_str().or(d["text"].as_str()).unwrap_or("an exception");
            let at = d["url"]
                .as_str()
                .filter(|u| !u.is_empty())
                .map(|u| format!(" at {u}:{}", d["lineNumber"].as_u64().unwrap_or(0) + 1))
                .unwrap_or_default();
            Some(format!("uncaught exception: {}{at}", cut(what.lines().next().unwrap_or(what))))
        }
        "Log.entryAdded" => {
            let e = &p["entry"];
            let level = e["level"].as_str()?;
            let source = e["source"].as_str().unwrap_or("");
            if !matches!(level, "error" | "warning") || !matches!(source, "network" | "security") {
                return None;
            }
            let url = e["url"].as_str().filter(|u| !u.is_empty()).map(|u| format!(" — {u}")).unwrap_or_default();
            Some(format!("{source} {level}: {}{url}", cut(e["text"].as_str().unwrap_or(""))))
        }
        _ => None,
    }
}

pub(in crate::core::agentbrowser::computer) async fn stop_cast(cdp: &Cdp, shared: &Shared) {
    let casting = shared.page.lock().ok().and_then(|mut p| p.casting.take());
    if let Some(s) = casting {
        let _ = cdp.call(Some(&s), "Page.stopScreencast", json!({}), Duration::from_secs(2)).await;
    }
}
