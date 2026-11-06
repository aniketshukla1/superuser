use super::*;

impl Computer {
    // -- the user's hands ------------------------------------------------------------------

    pub fn request_help(&self, reason: &str, session: &str) {
        if let Ok(mut c) = self.shared.control.lock() {
            c.request_help(reason, now());
        }
        audit(&self.shared, "computer.help_requested", reason, true, Some(session));
    }

    /// Ask for one value the assistant must never see, for the field this ref names.
    pub fn request_secret(&self, label: &str, r#ref: &str, snapshot_id: u64, session: &str) -> Result<(), String> {
        let el = self.resolve(r#ref, snapshot_id)?;
        if let Ok(mut c) = self.shared.control.lock() {
            c.request_secret(label, &el.r#ref, el.backend_node_id, now());
        }
        audit(&self.shared, "computer.secret_requested", label, true, Some(session));
        Ok(())
    }

    /// Counters a waiting tool reads: how many times control has been handed back, and how
    /// many secrets have landed.
    pub fn counters(&self) -> (u64, u64) {
        self.shared.control.lock().map(|c| (c.releases(), c.supplied())).unwrap_or((0, 0))
    }

    pub fn take_control(&self) {
        if let Ok(mut c) = self.shared.control.lock() {
            c.take(now());
        }
        audit(&self.shared, "computer.control_taken", "the user took control of the browser", true, None);
    }

    /// The user said "not now" to a request for help.
    pub fn decline_help(&self) -> bool {
        let declined = self.shared.control.lock().map(|mut c| c.decline(now())).unwrap_or(false);
        if declined {
            audit(&self.shared, "computer.help_declined", "the user said not now", true, None);
        }
        declined
    }

    pub fn declines(&self) -> u64 {
        self.shared.control.lock().map(|c| c.declines()).unwrap_or(0)
    }

    /// The user said no to the value they were asked for. The waiting tool sees the request
    /// gone and says so; nothing is typed.
    pub fn decline_secret(&self) -> bool {
        let had = self
            .shared
            .control
            .lock()
            .map(|mut c| {
                let had = c.pending_secret(now()).is_some();
                c.secret_abandoned();
                had
            })
            .unwrap_or(false);
        if had {
            audit(&self.shared, "computer.secret_declined", "the user didn't enter the value", true, None);
        }
        had
    }

    pub fn release_control(&self) {
        if let Ok(mut c) = self.shared.control.lock() {
            c.release(now());
        }
        audit(&self.shared, "computer.control_released", "the user handed the browser back", true, None);
    }

    /// Type the user's secret into the field it was asked for.
    ///
    /// The value is typed and forgotten: not stored, not returned, not logged. What comes
    /// back is how many characters arrived. It types and does not submit — sending the form
    /// is an action of its own, through the gateway.
    pub async fn supply_secret(&self, value: &str) -> Result<usize, String> {
        let pending = self
            .shared
            .control
            .lock()
            .map_err(|_| "control is unavailable".to_string())?
            .pending_secret(now())
            .ok_or("Nothing is waiting for a secret.")?;
        if value.is_empty() {
            return Err("Type the value first.".into());
        }
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let node = json!({"backendNodeId": pending.backend_node_id});
        let _ = cdp.call(Some(&session), "DOM.scrollIntoViewIfNeeded", node.clone(), ACTION_TIMEOUT).await;
        let object = match cdp.call(Some(&session), "DOM.resolveNode", node, ACTION_TIMEOUT).await {
            Ok(r) => r["object"]["objectId"].as_str().unwrap_or_default().to_string(),
            Err(_) => {
                // The field is gone and no retry brings it back. Close the request so the
                // assistant sees nothing is pending and can ask again from a fresh snapshot.
                if let Ok(mut c) = self.shared.control.lock() {
                    c.secret_abandoned();
                }
                return Err("Nothing was entered: that field has gone from the page. The assistant will need to ask for the value again.".into());
            }
        };
        cdp.call(
            Some(&session),
            "Runtime.callFunctionOn",
            json!({"objectId": object, "functionDeclaration": SELECT_CONTENTS, "returnByValue": true}),
            ACTION_TIMEOUT,
        )
        .await
        .map_err(words)?;
        cdp.call(Some(&session), "Input.insertText", json!({ "text": value }), ACTION_TIMEOUT).await.map_err(words)?;
        let _ = cdp.call(Some(&session), "Runtime.releaseObject", json!({"objectId": object}), ACTION_TIMEOUT).await;
        let chars = value.chars().count();
        if let Ok(mut c) = self.shared.control.lock() {
            c.secret_supplied();
        }
        audit(&self.shared, "computer.secret_supplied", &format!("{} entered ({chars} characters)", pending.label), true, None);
        Ok(chars)
    }

    /// One thing the user did on the live view. Applied only while they hold the wheel:
    /// a socket being open is not permission.
    pub async fn user_input(&self, m: &Value) -> Result<(), String> {
        if !self.shared.control.lock().map(|c| c.user_may_drive()).unwrap_or(false) {
            return Err(crate::core::agentbrowser::control::TAKE_CONTROL_FIRST.into());
        }
        self.touch();
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let (w, h) = chrome::VIEWPORT;
        // Clamped rather than refused: a click a pixel outside is a rounding artefact of
        // scaling the picture, not a mistake.
        let xy = |m: &Value| (m["x"].as_f64().unwrap_or(0.0).clamp(0.0, w as f64 - 1.0), m["y"].as_f64().unwrap_or(0.0).clamp(0.0, h as f64 - 1.0));
        let modifiers = m["modifiers"].as_i64().unwrap_or(0);
        match m["type"].as_str().unwrap_or("") {
            "mouse" => {
                let (x, y) = xy(m);
                let kind = match m["event"].as_str().unwrap_or("") {
                    "pressed" => "mousePressed",
                    "released" => "mouseReleased",
                    _ => "mouseMoved",
                };
                // A press or release needs a click count, or the page sees a move with a
                // button held and no click ever fires. The browser's own count makes a
                // double-click a double-click.
                let count = if kind == "mouseMoved" { 0 } else { m["clickCount"].as_i64().unwrap_or(1).max(1) };
                let button = m["button"].as_str().filter(|b| matches!(*b, "left" | "right" | "middle")).unwrap_or(if kind == "mouseMoved" { "none" } else { "left" });
                cdp.call(Some(&session), "Input.dispatchMouseEvent", json!({"type": kind, "x": x, "y": y, "button": button, "clickCount": count, "modifiers": modifiers}), ACTION_TIMEOUT).await.map_err(words)?;
            }
            "wheel" => {
                let (x, y) = xy(m);
                cdp.call(Some(&session), "Input.dispatchMouseEvent", json!({"type": "mouseWheel", "x": x, "y": y, "deltaX": m["deltaX"].as_f64().unwrap_or(0.0), "deltaY": m["deltaY"].as_f64().unwrap_or(0.0), "modifiers": modifiers}), ACTION_TIMEOUT).await.map_err(words)?;
            }
            "key" => {
                let key = m["key"].as_str().unwrap_or("");
                let mut p = keys::press(key).unwrap_or(keys::Press { key: key.into(), code: String::new(), vk: 0, text: None, modifiers: 0 });
                if let Some(code) = m["code"].as_str().filter(|c| !c.is_empty()) {
                    p.code = code.to_string();
                }
                if let Some(t) = m["text"].as_str().filter(|t| !t.is_empty()) {
                    p.text = Some(t.to_string());
                }
                p.modifiers = modifiers;
                if m["event"] == "up" {
                    cdp.call(Some(&session), "Input.dispatchKeyEvent", json!({"type": "keyUp", "key": p.key, "code": p.code, "windowsVirtualKeyCode": p.vk, "nativeVirtualKeyCode": p.vk, "modifiers": p.modifiers}), ACTION_TIMEOUT).await.map_err(words)?;
                } else {
                    let mut ev = json!({"type": if p.text.is_some() { "keyDown" } else { "rawKeyDown" }, "key": p.key, "code": p.code, "windowsVirtualKeyCode": p.vk, "nativeVirtualKeyCode": p.vk, "modifiers": p.modifiers});
                    if let Some(t) = &p.text {
                        ev["text"] = json!(t);
                    }
                    cdp.call(Some(&session), "Input.dispatchKeyEvent", ev, ACTION_TIMEOUT).await.map_err(words)?;
                }
            }
            // A paste, or a code the user didn't type one key at a time.
            "text" => {
                cdp.call(Some(&session), "Input.insertText", json!({"text": m["text"].as_str().unwrap_or("")}), ACTION_TIMEOUT).await.map_err(words)?;
            }
            other => return Err(format!("unknown input {other}")),
        }
        Ok(())
    }
}
