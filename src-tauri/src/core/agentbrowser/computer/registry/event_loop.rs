use super::super::*;

/// Chrome's side of the conversation: new windows, closed ones, navigations, dialogs and
/// screen frames.
pub(in crate::core::agentbrowser::computer) async fn event_loop(cdp: Arc<Cdp>, shared: Arc<Shared>) {
    let mut rx = cdp.subscribe();
    loop {
        let ev = match rx.recv().await {
            Ok(ev) => ev,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let current = shared.page.lock().map(|p| p.clone()).unwrap_or_default();
        let on_current = ev.session.as_deref() == Some(current.session.as_str());
        match ev.method.as_str() {
            // A site opening a window: the assistant follows it, as a person would see it.
            "Target.targetCreated" => {
                let info = &ev.params["targetInfo"];
                if info["type"] == "page" && info["targetId"].as_str() != Some(current.target.as_str()) && info["attached"] != true {
                    if let Some(t) = info["targetId"].as_str() {
                        let _ = attach(&cdp, &shared, t).await;
                    }
                }
            }
            "Target.targetDestroyed" | "Target.detachedFromTarget" => {
                let gone = ev.params["targetId"].as_str() == Some(current.target.as_str())
                    || ev.params["sessionId"].as_str() == Some(current.session.as_str());
                if gone && !current.target.is_empty() {
                    let targets = cdp.call(None, "Target.getTargets", json!({}), ACTION_TIMEOUT).await.unwrap_or_default();
                    let next = targets["targetInfos"].as_array().and_then(|a| {
                        a.iter()
                            .rev()
                            .find(|t| t["type"] == "page" && t["targetId"].as_str() != Some(current.target.as_str()))
                            .and_then(|t| t["targetId"].as_str())
                            .map(str::to_string)
                    });
                    let next = match next {
                        Some(t) => Some(t),
                        None => cdp
                            .call(None, "Target.createTarget", json!({"url": "about:blank"}), ACTION_TIMEOUT)
                            .await
                            .ok()
                            .and_then(|r| r["targetId"].as_str().map(str::to_string)),
                    };
                    if let Some(t) = next {
                        let _ = attach(&cdp, &shared, &t).await;
                    }
                }
            }
            "Page.frameNavigated" if on_current && ev.params["frame"]["parentId"].is_null() => {
                if let Ok(mut r) = shared.refs.lock() {
                    r.bump();
                    if let Some(u) = ev.params["frame"]["url"].as_str() {
                        r.url = u.to_string();
                    }
                }
            }
            "Page.navigatedWithinDocument" if on_current => {
                if let Ok(mut r) = shared.refs.lock() {
                    r.bump();
                    if let Some(u) = ev.params["url"].as_str() {
                        r.url = u.to_string();
                    }
                }
            }
            "Runtime.consoleAPICalled" | "Runtime.exceptionThrown" | "Log.entryAdded" if on_current => {
                if let Some(note) = note_of(&ev.method, &ev.params) {
                    if let Ok(mut n) = shared.notes.lock() {
                        if n.len() >= NOTES_CAP {
                            n.pop_front();
                        }
                        n.push_back(note);
                    }
                }
            }
            // An alert would freeze the page for every later action. Alerts and "leave
            // this page?" are accepted; a confirm or a prompt is declined, because saying
            // yes to one is a decision.
            "Page.javascriptDialogOpening" => {
                let kind = ev.params["type"].as_str().unwrap_or("");
                let accept = matches!(kind, "alert" | "beforeunload");
                if let Some(s) = ev.session.as_deref() {
                    let _ = cdp.call(Some(s), "Page.handleJavaScriptDialog", json!({"accept": accept}), ACTION_TIMEOUT).await;
                }
            }
            "Page.screencastFrame" => {
                if let Some(s) = ev.session.as_deref() {
                    // Every frame acknowledged: Chrome sends the next only after the last is
                    // acked, which is the backpressure that keeps a slow viewer from drowning.
                    let _ = cdp.call(Some(s), "Page.screencastFrameAck", json!({"sessionId": ev.params["sessionId"]}), ACTION_TIMEOUT).await;
                }
                if on_current {
                    let meta = &ev.params["metadata"];
                    let _ = shared.screen.send(Screen::Frame {
                        data: ev.params["data"].as_str().unwrap_or_default().to_string(),
                        width: meta["deviceWidth"].as_f64().unwrap_or(chrome::VIEWPORT.0 as f64) as u32,
                        height: meta["deviceHeight"].as_f64().unwrap_or(chrome::VIEWPORT.1 as f64) as u32,
                    });
                }
            }
            _ => {}
        }
    }
}

pub(in crate::core::agentbrowser::computer) enum Waited {
    Arrived,
    TimedOut,
    Stopped,
    Closed,
}

pub(in crate::core::agentbrowser::computer) async fn wait_for(rx: &mut broadcast::Receiver<Event>, session: &str, methods: &[&str], timeout: Duration, cancel: Option<&AtomicBool>) -> Waited {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
            return Waited::Stopped;
        }
        if tokio::time::Instant::now() >= deadline {
            return Waited::TimedOut;
        }
        match tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
            Ok(Ok(ev)) if ev.session.as_deref() == Some(session) && methods.contains(&ev.method.as_str()) => return Waited::Arrived,
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) | Err(_) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => return Waited::Closed,
        }
    }
}

pub(in crate::core::agentbrowser::computer) async fn read_page(cdp: &Cdp, session: &str) -> Result<PageText, String> {
    let mut last = String::new();
    // Twice: a page mid-navigation answers "execution context was destroyed" once.
    for _ in 0..2 {
        match cdp
            .call(Some(session), "Runtime.evaluate", json!({"expression": READ_SCRIPT, "returnByValue": true}), ACTION_TIMEOUT)
            .await
        {
            Ok(r) => {
                let v: Value = serde_json::from_str(r["result"]["value"].as_str().unwrap_or("{}")).unwrap_or_default();
                let raw = v["text"].as_str().unwrap_or("");
                let mut collapsed = String::with_capacity(raw.len().min(TEXT_LIMIT + 64));
                let mut blank = 0;
                for line in raw.lines() {
                    if line.trim().is_empty() {
                        blank += 1;
                        if blank > 1 {
                            continue;
                        }
                    } else {
                        blank = 0;
                    }
                    collapsed.push_str(line.trim_end());
                    collapsed.push('\n');
                }
                let collapsed = collapsed.trim().to_string();
                let truncated = collapsed.chars().count() > TEXT_LIMIT;
                return Ok(PageText {
                    url: v["url"].as_str().unwrap_or("").to_string(),
                    title: v["title"].as_str().unwrap_or("").to_string(),
                    text: collapsed.chars().take(TEXT_LIMIT).collect(),
                    truncated,
                });
            }
            Err(e) => {
                last = e;
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
    Err(words(last))
}

pub(in crate::core::agentbrowser::computer) async fn capture(cdp: &Cdp, session: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let shot = cdp
        .call(Some(session), "Page.captureScreenshot", json!({"format": "jpeg", "quality": 70}), ACTION_TIMEOUT)
        .await
        .map_err(words)?;
    base64::engine::general_purpose::STANDARD
        .decode(shot["data"].as_str().unwrap_or(""))
        .map_err(|e| format!("the screenshot wasn't readable: {e}"))
}
