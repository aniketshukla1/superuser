use super::*;

impl Computer {
    // -- acting ----------------------------------------------------------------------

    pub async fn click(&self, r#ref: &str, snapshot_id: u64, cancel: Option<&AtomicBool>) -> Result<String, String> {
        self.may_act()?;
        self.touch();
        let el = self.resolve(r#ref, snapshot_id)?;
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let before = self.shared.refs.lock().map(|r| (r.generation, r.url.clone())).unwrap_or_default();
        let (x, y) = centre(&cdp, &session, &el).await?;
        if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
            return Err("Stopped before clicking.".into());
        }
        for (kind, count) in [("mouseMoved", 0), ("mousePressed", 1), ("mouseReleased", 1)] {
            cdp.call(
                Some(&session),
                "Input.dispatchMouseEvent",
                json!({"type": kind, "x": x, "y": y, "button": "left", "clickCount": count}),
                ACTION_TIMEOUT,
            )
            .await
            .map_err(words)?;
        }
        Ok(format!("Clicked {} “{}”.{}", el.role, el.name, self.after_action(&cdp, before).await))
    }

    /// Put text in a field — replacing what is there — or choose a dropdown's option.
    pub async fn type_text(&self, r#ref: &str, snapshot_id: u64, text: &str, submit: bool, cancel: Option<&AtomicBool>) -> Result<String, String> {
        self.may_act()?;
        self.touch();
        let el = self.resolve(r#ref, snapshot_id)?;
        if el.input_type == "password" {
            return Err(format!(
                "“{}” is a password field. Don't type into it — call browser_request_secret with ref {} and a short label, so the user can enter it without you ever seeing it.",
                el.name, el.r#ref
            ));
        }
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let before = self.shared.refs.lock().map(|r| (r.generation, r.url.clone())).unwrap_or_default();
        if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
            return Err("Stopped before typing.".into());
        }
        let _ = cdp.call(Some(&session), "DOM.scrollIntoViewIfNeeded", json!({"backendNodeId": el.backend_node_id}), ACTION_TIMEOUT).await;
        let object = cdp
            .call(Some(&session), "DOM.resolveNode", json!({"backendNodeId": el.backend_node_id}), ACTION_TIMEOUT)
            .await
            .map_err(words)?["object"]["objectId"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let said = if el.input_type == "select" {
            let r = cdp
                .call(
                    Some(&session),
                    "Runtime.callFunctionOn",
                    json!({"objectId": object, "functionDeclaration": CHOOSE_OPTION, "arguments": [{"value": text}], "returnByValue": true}),
                    ACTION_TIMEOUT,
                )
                .await
                .map_err(words)?;
            let v: Value = serde_json::from_str(r["result"]["value"].as_str().unwrap_or("{}")).unwrap_or_default();
            if v["ok"] != true {
                let options: Vec<String> = v["options"].as_array().map(|a| a.iter().filter_map(|o| o.as_str().map(String::from)).collect()).unwrap_or_default();
                return Err(format!("“{}” has no option “{text}”. Its options are: {}.", el.name, options.join(", ")));
            }
            format!("Chose “{}” in “{}”", v["label"].as_str().unwrap_or(text), el.name)
        } else {
            cdp.call(
                Some(&session),
                "Runtime.callFunctionOn",
                json!({"objectId": object, "functionDeclaration": SELECT_CONTENTS, "returnByValue": true}),
                ACTION_TIMEOUT,
            )
            .await
            .map_err(words)?;
            if text.is_empty() {
                press_key(&cdp, &session, &keys::press("Delete").expect("a known key")).await?;
            } else {
                cdp.call(Some(&session), "Input.insertText", json!({"text": text}), ACTION_TIMEOUT).await.map_err(words)?;
            }
            // The count, never the text: this result is read by the model and may be logged,
            // and a value typed into a form is where a card number lives.
            format!("Filled {} “{}” with {} characters", el.role, el.name, text.chars().count())
        };
        let _ = cdp.call(Some(&session), "Runtime.releaseObject", json!({"objectId": object}), ACTION_TIMEOUT).await;
        if submit {
            press_key(&cdp, &session, &keys::press("Enter").expect("a known key")).await?;
        }
        Ok(format!("{said}{}.{}", if submit { " and pressed Enter" } else { "" }, self.after_action(&cdp, before).await))
    }

    pub async fn key(&self, spec: &str, target: Option<(&str, u64)>) -> Result<String, String> {
        self.may_act()?;
        self.touch();
        let press = keys::press(spec).ok_or_else(|| {
            format!("“{spec}” isn't a key name. Use names like Enter, Tab, Escape, ArrowDown or Backspace, a single character, or a combination such as Control+A.")
        })?;
        let el = match target {
            Some((r, id)) => Some(self.resolve(r, id)?),
            None => None,
        };
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let before = self.shared.refs.lock().map(|r| (r.generation, r.url.clone())).unwrap_or_default();
        if let Some(el) = &el {
            cdp.call(Some(&session), "DOM.focus", json!({"backendNodeId": el.backend_node_id}), ACTION_TIMEOUT).await.map_err(words)?;
        }
        press_key(&cdp, &session, &press).await?;
        let on = el.map(|e| format!(" on “{}”", e.name)).unwrap_or_default();
        Ok(format!("Pressed {spec}{on}.{}", self.after_action(&cdp, before).await))
    }

    pub async fn scroll(&self, down: bool, pages: u32) -> Result<String, String> {
        self.may_act()?;
        self.touch();
        let pages = pages.clamp(1, 10);
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let (w, h) = chrome::VIEWPORT;
        let delta = (h as f64 * 0.75 * pages as f64) * if down { 1.0 } else { -1.0 };
        cdp.call(
            Some(&session),
            "Input.dispatchMouseEvent",
            json!({"type": "mouseWheel", "x": w / 2, "y": h / 2, "deltaX": 0, "deltaY": delta}),
            ACTION_TIMEOUT,
        )
        .await
        .map_err(words)?;
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok(format!(
            "Scrolled {} {}. Take a new snapshot to see what is in view now.",
            if down { "down" } else { "up" },
            if pages == 1 { "one page".to_string() } else { format!("{pages} pages") }
        ))
    }

    /// After a click or a keystroke: wait a moment for whatever it started, then say
    /// where the page is — and that the refs are gone, when they are.
    pub(in crate::core::agentbrowser::computer) async fn after_action(&self, cdp: &Cdp, before: (u64, String)) -> String {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let session = self.page().session;
        let Ok(now_page) = read_page(cdp, &session).await else { return String::new() };
        self.remember_page(&now_page);
        let generation = self.shared.refs.lock().map(|r| r.generation).unwrap_or(0);
        if generation != before.0 || now_page.url != before.1 {
            format!(" The page is now “{}” ({}). Take a new snapshot before acting again.", now_page.title, now_page.url)
        } else {
            String::new()
        }
    }

    // -- a picture ---------------------------------------------------------------------

    pub async fn screenshot(&self) -> Result<Vec<u8>, String> {
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        capture(&cdp, &self.page().session).await
    }

    /// Keep what the page looked like, so a finished turn shows the page it opened rather
    /// than whatever the browser shows today — "a record must not change its mind".
    pub async fn keep_frame(&self) -> Option<String> {
        let bytes = self.screenshot().await.ok()?;
        if bytes.len() > FRAME_MAX_BYTES {
            return None;
        }
        let id = format!("f-{:x}-{:04x}", (now() * 1000.0) as u64, rand::random::<u16>());
        let dir = frames_dir(&self.shared.data_dir, &self.shared.persona);
        std::fs::create_dir_all(&dir).ok()?;
        let path = dir.join(format!("{id}.jpg"));
        // Write-once: a frame is never overwritten.
        if path.exists() {
            return None;
        }
        std::fs::write(&path, bytes).ok()?;
        Some(id)
    }
}
