use super::*;

impl Computer {
    pub fn persona(&self) -> &str {
        &self.shared.persona
    }

    pub(in crate::core::agentbrowser::computer) fn touch(&self) {
        if let Ok(mut t) = self.shared.last_used.lock() {
            *t = now();
        }
    }

    pub fn last_used(&self) -> f64 {
        self.shared.last_used.lock().map(|t| *t).unwrap_or(0.0)
    }

    pub async fn is_running(&self) -> bool {
        let guard = self.running.lock().await;
        guard.as_ref().is_some_and(|r| !r.cdp.is_closed())
    }

    pub fn control(&self) -> ControlView {
        self.shared.control.lock().map(|mut c| c.view(now())).unwrap_or_else(|_| Control::new(now()).view(now()))
    }

    pub(in crate::core::agentbrowser::computer) fn may_act(&self) -> Result<(), String> {
        self.shared.control.lock().map_err(|_| "control is unavailable".to_string())?.bot_may_act().map_err(str::to_string)
    }

    pub(in crate::core::agentbrowser::computer) fn page(&self) -> Page {
        self.shared.page.lock().map(|p| p.clone()).unwrap_or_default()
    }

    // -- starting and stopping -----------------------------------------------------

    /// The browser, started if it isn't running. Called with the action lock held.
    pub(in crate::core::agentbrowser::computer) async fn ensure(&self, guard: &mut Option<Running>) -> Result<Arc<Cdp>, String> {
        if let Some(r) = guard.as_mut() {
            let exited = r.child.try_wait().ok().flatten().is_some();
            if !exited && !r.cdp.is_closed() {
                return Ok(r.cdp.clone());
            }
            // It died underneath us. Clean up what is left and start again.
            if let Some(old) = guard.take() {
                old.events.abort();
                old.proxy.stop();
            }
            self.forget_pages("The browser stopped unexpectedly.");
        }
        make_room(&self.shared.persona);

        let binary = chrome::find().ok_or(
            "No Chrome-family browser was found on this Mac. Install Google Chrome or Chromium, then try again.",
        )?;
        let data_dir = &self.shared.data_dir;
        let persona = &self.shared.persona;
        let profile = profile_dir(data_dir, persona);
        std::fs::create_dir_all(&profile).map_err(|e| format!("couldn't create the browser profile: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(root(data_dir, persona), std::fs::Permissions::from_mode(0o700));
        }
        let _ = std::fs::create_dir_all(frames_dir(data_dir, persona));
        purge_frames(&frames_dir(data_dir, persona));
        chrome::clear_stale_locks(&profile);

        let proxy = proxy::start(persona, data_dir.clone(), allow_loopback().clone())
            .await
            .map_err(|e| format!("couldn't start the browser's network proxy: {e}"))?;

        #[cfg(not(unix))]
        {
            let _ = (binary, proxy);
            return Err("The assistant's browser needs macOS or Linux.".into());
        }
        #[cfg(unix)]
        {
            let args = chrome::args(&profile, proxy.port, !headed().load(Ordering::SeqCst));
            let log = root(data_dir, persona).join("chrome.log");
            let launched = chrome::launch(&binary, &args, &log).map_err(|e| format!("couldn't start {}: {e}", binary.display()))?;
            self.shared.pid.store(launched.pid, Ordering::SeqCst);
            let cdp = Cdp::start(launched.from_chrome, launched.to_chrome);
            let mut child = launched.child;

            if let Err(e) = cdp.call(None, "Browser.getVersion", json!({}), START_TIMEOUT).await {
                chrome::kill_group(self.shared.pid.load(Ordering::SeqCst));
                let _ = child.kill().await;
                let tail = std::fs::read_to_string(&log).unwrap_or_default();
                let last = tail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").chars().take(200).collect::<String>();
                return Err(format!("The browser didn't start ({e}).{}", if last.is_empty() { String::new() } else { format!(" Its log says: {last}") }));
            }
            // Downloads go nowhere: a site cannot put a file on this machine through an
            // assistant, which is a thing a person must decide to do.
            let _ = cdp.call(None, "Browser.setDownloadBehavior", json!({"behavior": "deny"}), ACTION_TIMEOUT).await;
            let _ = cdp.call(None, "Target.setDiscoverTargets", json!({"discover": true}), ACTION_TIMEOUT).await;

            let targets = cdp.call(None, "Target.getTargets", json!({}), ACTION_TIMEOUT).await.unwrap_or_default();
            let first = targets["targetInfos"]
                .as_array()
                .and_then(|a| a.iter().find(|t| t["type"] == "page").and_then(|t| t["targetId"].as_str()).map(str::to_string));
            let target = match first {
                Some(t) => t,
                None => cdp
                    .call(None, "Target.createTarget", json!({"url": "about:blank"}), ACTION_TIMEOUT)
                    .await?["targetId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            };
            attach(&cdp, &self.shared, &target).await?;

            let events = tokio::spawn(event_loop(cdp.clone(), self.shared.clone()));
            *guard = Some(Running { child, cdp: cdp.clone(), proxy, events });
            Ok(cdp)
        }
    }

    pub(in crate::core::agentbrowser::computer) fn forget_pages(&self, reason: &str) {
        if let Ok(mut p) = self.shared.page.lock() {
            *p = Page::default();
        }
        if let Ok(mut r) = self.shared.refs.lock() {
            r.bump();
            r.elements.clear();
            r.snapshot = 0;
        }
        let _ = self.shared.screen.send(Screen::Stopped { reason: reason.to_string() });
    }

    /// Stop the browser and keep what it knows — its sign-ins survive the next start.
    pub async fn stop(&self, reason: &str) -> bool {
        // Killed before the lock is waited for, so Stop lands even when an action is
        // mid-navigation; the action then fails with "the browser closed" and lets go.
        let pid = self.shared.pid.swap(0, Ordering::SeqCst);
        let mut guard = self.running.lock().await;
        let Some(mut r) = guard.take() else {
            return false;
        };
        // Gracefully first, so Chrome flushes its cookies to the profile.
        let _ = r.cdp.call(None, "Browser.close", json!({}), Duration::from_secs(2)).await;
        let exited = tokio::time::timeout(Duration::from_secs(3), r.child.wait()).await.is_ok();
        if !exited {
            chrome::kill_group(pid);
            let _ = r.child.kill().await;
        }
        r.events.abort();
        r.proxy.stop();
        self.forget_pages(reason);
        if let Ok(mut c) = self.shared.control.lock() {
            // Whoever had the wheel of a browser that no longer exists, has nothing.
            c.release(now());
        }
        audit(&self.shared, "computer.stopped", reason, true, None);
        true
    }

    /// Sign out of everything: stop, then delete the profile and every kept frame. Written
    /// down at the point of no return, before anything is deleted.
    pub async fn reset(&self) -> Result<(), String> {
        self.stop("Signed out of everything.").await;
        audit(&self.shared, "computer.reset", "the profile and its kept screens were deleted", true, None);
        let dir = root(&self.shared.data_dir, &self.shared.persona);
        for d in [profile_dir(&self.shared.data_dir, &self.shared.persona), frames_dir(&self.shared.data_dir, &self.shared.persona)] {
            if d.exists() {
                std::fs::remove_dir_all(&d).map_err(|e| format!("couldn't delete {}: {e}", d.display()))?;
            }
        }
        let _ = std::fs::remove_file(dir.join("chrome.log"));
        Ok(())
    }

    pub async fn status(&self) -> Status {
        let running = self.is_running().await;
        let (url, title) = self.shared.refs.lock().map(|r| (r.url.clone(), r.title.clone())).unwrap_or_default();
        Status {
            persona: self.shared.persona.clone(),
            running,
            url: if running { url } else { String::new() },
            title: if running { title } else { String::new() },
            control: self.control(),
            has_profile: profile_dir(&self.shared.data_dir, &self.shared.persona).exists(),
            viewers: self.shared.viewers.load(Ordering::SeqCst),
            last_used: self.last_used(),
        }
    }
}
