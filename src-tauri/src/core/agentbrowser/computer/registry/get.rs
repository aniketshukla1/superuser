use super::super::*;

pub(in crate::core::agentbrowser::computer) fn registry() -> &'static Mutex<HashMap<String, Arc<Computer>>> {
    static R: OnceLock<Mutex<HashMap<String, Arc<Computer>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Whether the browser may reach this machine's own services. Set from Settings.
pub fn allow_loopback() -> &'static Arc<AtomicBool> {
    static A: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    A.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// Whether to show the browser window rather than run it headless. Set from Settings.
pub fn headed() -> &'static AtomicBool {
    static H: AtomicBool = AtomicBool::new(false);
    &H
}

/// The computer for `persona`, created on first ask. Nothing is launched until an action.
pub fn get(data_dir: &Path, persona: &str) -> Result<Arc<Computer>, String> {
    if !is_plain_id(persona) {
        return Err(format!("“{persona}” isn't a usable assistant id for a browser."));
    }
    let mut r = registry().lock().map_err(|_| "the browser registry is unavailable".to_string())?;
    Ok(r.entry(persona.to_string())
        .or_insert_with(|| {
            let (screen, _) = broadcast::channel(8);
            Arc::new(Computer {
                shared: Arc::new(Shared {
                    persona: persona.to_string(),
                    data_dir: data_dir.to_path_buf(),
                    refs: Mutex::new(Refs::default()),
                    control: Mutex::new(Control::new(now())),
                    page: Mutex::new(Page::default()),
                    screen,
                    viewers: AtomicUsize::new(0),
                    last_used: Mutex::new(now()),
                    pid: AtomicU32::new(0),
                    notes: Mutex::new(VecDeque::new()),
                }),
                running: tokio::sync::Mutex::new(None),
            })
        })
        .clone())
}

/// The computer for `persona` if one has been created.
pub fn existing(persona: &str) -> Option<Arc<Computer>> {
    registry().lock().ok()?.get(persona).cloned()
}

pub fn all() -> Vec<Arc<Computer>> {
    registry().lock().map(|r| r.values().cloned().collect()).unwrap_or_default()
}

/// The element a ref names in the persona's latest snapshot, and the page it is on — what
/// `core::gateway` judges a click or a keystroke on. Resolved from THIS process's own
/// snapshot, never from anything the model said about the element, so "never click
/// Submit" cannot be stepped around by calling the button something else.
pub fn element_for(persona: &str, r#ref: &str) -> Option<(Element, String)> {
    let c = existing(persona)?;
    let refs = c.shared.refs.lock().ok()?;
    let el = refs.elements.iter().find(|e| e.r#ref == r#ref)?.clone();
    Some((el, refs.url.clone()))
}

/// The page the persona's browser is on, as last seen.
pub fn current_url(persona: &str) -> Option<String> {
    let c = existing(persona)?;
    let url = c.shared.refs.lock().ok()?.url.clone();
    (!url.is_empty()).then_some(url)
}

/// Stop every browser as the app quits: gracefully, all at once, for as long as `grace`
/// allows — a graceful close is what writes the latest sign-ins to the profile — and then
/// killed, so no Chrome outlives the app that started it.
pub async fn stop_all(grace: Duration) {
    let everyone = all();
    let stops = everyone.iter().map(|c| c.stop("The app closed."));
    let _ = tokio::time::timeout(grace, futures_util::future::join_all(stops)).await;
    for c in everyone {
        chrome::kill_group(c.shared.pid.swap(0, Ordering::SeqCst));
    }
}

/// Close browsers nobody has used for half an hour. Called on a timer by the app.
pub async fn sweep_idle() {
    for c in all() {
        let idle = now() - c.last_used() > IDLE_SECS;
        let driving = c.shared.control.lock().map(|k| k.holder() == Holder::User).unwrap_or(false);
        if idle && !driving && c.shared.viewers.load(Ordering::SeqCst) == 0 && c.is_running().await {
            c.stop("It hadn't been used for 30 minutes.").await;
        }
    }
}

pub(in crate::core::agentbrowser::computer) fn audit(shared: &Shared, event: &str, detail: &str, ok: bool, session: Option<&str>) {
    let mut e = crate::core::audit::Entry::event(event);
    e.actor = shared.persona.clone();
    e.persona = shared.persona.clone();
    if let Some(s) = session {
        e.session = s.to_string();
        e.initiator = Some(crate::core::audit::Initiator::from_session(s));
    } else {
        e.initiator = Some(crate::core::audit::Initiator::new("person", ""));
    }
    e.detail = detail.to_string();
    e.ok = ok;
    crate::core::audit::append(&shared.data_dir, e);
}

/// A CDP error, in words a model can act on.
pub(in crate::core::agentbrowser::computer) fn words(err: String) -> String {
    let low = err.to_lowercase();
    if low.contains("no node") || low.contains("could not find node") || low.contains("detached") || low.contains("does not belong to the document") {
        "That element is no longer on the page. Take a new snapshot and pick a ref from it.".into()
    } else if err == crate::core::agentbrowser::cdp::CLOSED {
        "Your browser closed. Open the page again with browser_navigate.".into()
    } else {
        err
    }
}

pub(in crate::core::agentbrowser::computer) const READ_SCRIPT: &str = r#"(() => {
  const b = document.body;
  return JSON.stringify({ url: location.href, title: document.title, text: b ? (b.innerText || '') : '' });
})()"#;

/// Select a field's current contents, so the text typed next replaces them — what "put
/// this value in this box" means. Without it a field half-filled by an earlier attempt
/// ends up holding "AlicAlice".
pub(in crate::core::agentbrowser::computer) const SELECT_CONTENTS: &str = r#"function () {
  this.focus();
  if (typeof this.select === 'function') { this.select(); return 'select'; }
  if (this.isContentEditable) {
    const r = document.createRange(); r.selectNodeContents(this);
    const s = getSelection(); s.removeAllRanges(); s.addRange(r); return 'range';
  }
  return 'none';
}"#;

pub(in crate::core::agentbrowser::computer) const CHOOSE_OPTION: &str = r#"function (want) {
  const w = String(want).trim().toLowerCase();
  const opts = Array.from(this.options || []);
  const o = opts.find(o => o.label.trim().toLowerCase() === w)
    || opts.find(o => o.value.trim().toLowerCase() === w)
    || opts.find(o => o.label.trim().toLowerCase().includes(w));
  if (!o) return JSON.stringify({ ok: false, options: opts.slice(0, 30).map(o => o.label.trim()) });
  this.value = o.value;
  this.dispatchEvent(new Event('input', { bubbles: true }));
  this.dispatchEvent(new Event('change', { bubbles: true }));
  return JSON.stringify({ ok: true, label: o.label.trim() });
}"#;
