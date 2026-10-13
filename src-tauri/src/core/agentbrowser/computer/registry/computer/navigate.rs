use super::*;

impl Computer {
    // -- reading ---------------------------------------------------------------------

    /// Open a page and read it.
    pub async fn navigate(&self, url: &str, cancel: Option<&AtomicBool>) -> Result<PageText, String> {
        self.may_act()?;
        self.go(url, cancel).await
    }

    /// Open a page for the user — to sign in to a site before the assistant needs it — with
    /// the wheel theirs before the page loads, so there is no moment the assistant could act
    /// on a half-finished sign-in. The same floors as any navigation: this is still the
    /// assistant's browser, behind its proxy.
    pub async fn open_for_user(&self, url: &str) -> Result<PageText, String> {
        if self.control().holder != Holder::User {
            self.take_control();
        }
        self.go(url, None).await
    }

    pub(in crate::core::agentbrowser::computer) async fn go(&self, url: &str, cancel: Option<&AtomicBool>) -> Result<PageText, String> {
        let host = crate::core::boundaries::host_of(url);
        if !(url.starts_with("http://") || url.starts_with("https://")) || host.is_empty() {
            return Err("Your browser only opens http and https addresses.".into());
        }
        // Said here, before a browser is started, for the one navigation the model asked
        // for by name. The proxy still holds the same line for everything the page loads —
        // but for a plain-HTTP page Chrome draws the proxy's refusal as page content, and
        // "the page says Refused by Superuser" is not a sentence a model reliably reports.
        let port = reqwest::Url::parse(url).ok().and_then(|u| u.port_or_known_default()).unwrap_or(0);
        let staging = crate::core::repro::is_private_staging(&self.shared.persona, &host, port);
        let loopback = proxy::decide_name_or_staging(&host, allow_loopback().load(Ordering::SeqCst), staging)
            .map_err(|reason| format!("Refused. Your browser was stopped from reaching {host}: {reason}."))?;
        if !loopback && crate::core::egress::local_only() {
            return Err(format!(
                "Local-only mode is on, so your browser can't reach {host}. Nothing was opened. The user can turn local-only off in Settings to let you browse."
            ));
        }
        self.touch();
        let mut guard = self.running.lock().await;
        let cdp = self.ensure(&mut guard).await?;
        let page = self.page();
        let mut rx = cdp.subscribe();
        let asked_at = now() - 0.5;
        let started = cdp
            .call_with(Some(&page.session), "Page.navigate", json!({ "url": url }), NAVIGATION_TIMEOUT, cancel)
            .await
            .map_err(words)?;
        if let Ok(mut r) = self.shared.refs.lock() {
            r.bump();
        }
        let refused_since = |guard: &Option<Running>, hosts: &[&str]| {
            guard.as_ref().and_then(|r| r.proxy.refusals().into_iter().rev().find(|x| x.at >= asked_at && hosts.contains(&x.host.as_str())))
        };
        if let Some(err) = started["errorText"].as_str().filter(|e| !e.is_empty()) {
            return Err(match refused_since(&guard, &[host.as_str()]) {
                Some(x) => format!("Refused. Your browser was stopped from reaching {}: {}.", x.host, x.reason),
                None => format!("Couldn't open {url}: {err}."),
            });
        }
        let mut note = String::new();
        // A same-document navigation (a #fragment) has no loader and fires nothing to wait for.
        if started["loaderId"].as_str().is_some_and(|l| !l.is_empty()) {
            let loaded = wait_for(&mut rx, &page.session, &["Page.domContentEventFired", "Page.loadEventFired"], NAVIGATION_TIMEOUT, cancel).await;
            match loaded {
                Waited::Arrived => {}
                Waited::Stopped => return Err(crate::core::agentbrowser::cdp::STOPPED.into()),
                Waited::TimedOut => note = "The page was still loading after 30 seconds; this is what it showed.".into(),
                Waited::Closed => return Err(words(crate::core::agentbrowser::cdp::CLOSED.into())),
            }
        }
        let mut text = read_page(&cdp, &page.session).await?;
        // A redirect to a host the proxy refused lands on the proxy's refusal page rather
        // than on an error. That is still a refusal, and it is reported as one.
        let landed = crate::core::boundaries::host_of(&text.url);
        if let Some(x) = refused_since(&guard, &[host.as_str(), landed.as_str()]) {
            self.remember_page(&text);
            return Err(format!("Refused. Your browser was stopped from reaching {}: {}.", x.host, x.reason));
        }
        if !note.is_empty() {
            text.text = format!("({note})\n\n{}", text.text);
        }
        self.remember_page(&text);
        Ok(text)
    }

    pub(in crate::core::agentbrowser::computer) fn remember_page(&self, p: &PageText) {
        if let Ok(mut r) = self.shared.refs.lock() {
            r.url = p.url.clone();
            r.title = p.title.clone();
        }
    }

    /// The page as it is now, without navigating.
    /// What the page reported on its own since the last look — taken, so the next look starts
    /// empty — and the browser and system it ran on, for a bug report.
    pub async fn console(&self) -> Result<(String, Vec<String>), String> {
        self.touch();
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let product = cdp
            .call(None, "Browser.getVersion", json!({}), ACTION_TIMEOUT)
            .await
            .ok()
            .and_then(|v| v["product"].as_str().map(str::to_string))
            .unwrap_or_else(|| "Chrome".into());
        let os = match std::env::consts::OS {
            "macos" => "macOS",
            "linux" => "Linux",
            "windows" => "Windows",
            other => other,
        };
        let notes = self.shared.notes.lock().map(|mut n| n.drain(..).collect()).unwrap_or_default();
        Ok((format!("{product} on {os} ({})", std::env::consts::ARCH), notes))
    }

    pub async fn read(&self) -> Result<PageText, String> {
        self.touch();
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let text = read_page(&cdp, &self.page().session).await?;
        self.remember_page(&text);
        Ok(text)
    }

    /// Everything on the page an assistant can act on, as the model reads it.
    pub async fn snapshot(&self) -> Result<String, String> {
        self.touch();
        let guard = self.running.lock().await;
        let cdp = open_cdp(&guard)?;
        let session = self.page().session;
        let tree = cdp.call(Some(&session), "Accessibility.getFullAXTree", json!({}), ACTION_TIMEOUT).await.map_err(words)?;
        let nodes = tree["nodes"].as_array().cloned().unwrap_or_default();
        let (mut elements, truncated) = snapshot::elements_from_ax(&nodes);
        // The DOM `type` of the fields a value can be typed into — the one fact the
        // accessibility tree leaves out that matters: which ones are passwords.
        let mut asked = 0;
        for el in elements.iter_mut() {
            if !snapshot::TYPABLE.contains(&el.role.as_str()) || asked >= 60 {
                continue;
            }
            asked += 1;
            if let Ok(d) = cdp.call(Some(&session), "DOM.describeNode", json!({"backendNodeId": el.backend_node_id}), ACTION_TIMEOUT).await {
                let node = &d["node"];
                if node["nodeName"] == "SELECT" {
                    el.input_type = "select".into();
                } else if let Some(attrs) = node["attributes"].as_array() {
                    for pair in attrs.chunks(2) {
                        if pair.first().and_then(|k| k.as_str()) == Some("type") {
                            el.input_type = pair.get(1).and_then(|v| v.as_str()).unwrap_or("").to_ascii_lowercase();
                        }
                    }
                }
            }
        }
        let page = read_page(&cdp, &session).await.unwrap_or(PageText { url: String::new(), title: String::new(), text: String::new(), truncated: false });
        let id = {
            let mut r = self.shared.refs.lock().map_err(|_| "snapshot state unavailable".to_string())?;
            r.bump();
            r.snapshot = r.generation;
            r.elements = elements.clone();
            r.url = page.url.clone();
            r.title = page.title.clone();
            r.generation
        };
        Ok(snapshot::render(id, &page.title, &page.url, &elements, truncated))
    }

    pub(in crate::core::agentbrowser::computer) fn resolve(&self, r#ref: &str, snapshot_id: u64) -> Result<Element, String> {
        let refs = self.shared.refs.lock().map_err(|_| "snapshot state unavailable".to_string())?;
        if refs.snapshot == 0 {
            return Err("Take a snapshot first (browser_snapshot), then use the refs it returns.".into());
        }
        if snapshot_id != refs.generation {
            return Err(format!(
                "Those refs are out of date: they came from snapshot {snapshot_id} and the page is now at {}. Take a new snapshot and use its refs.",
                refs.generation
            ));
        }
        refs.elements
            .iter()
            .find(|e| e.r#ref == r#ref)
            .cloned()
            .ok_or_else(|| format!("Nothing on this page has the ref {ref}. Take a new snapshot and use the refs it returns.", ref = r#ref))
    }
}
