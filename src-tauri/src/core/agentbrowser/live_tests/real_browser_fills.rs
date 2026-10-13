use super::*;

pub(super) const FORM: &str = r#"<!doctype html><html><head><title>Sign up</title></head><body>
<h1>Join the newsletter</h1>
<form action="/done" method="get">
  <label>Name <input name="name" type="text"></label>
  <label>Password <input name="pw" type="password" id="pw"></label>
  <label>Country <select name="country"><option value="">Choose</option><option value="CA">Canada</option><option value="FR">France</option></select></label>
  <label><input type="checkbox" name="subscribe"> Subscribe</label>
  <button type="submit">Send</button>
</form>
<p id="len">length: 0</p>
<script>document.getElementById('pw').addEventListener('input', e => { document.getElementById('len').textContent = 'length: ' + e.target.value.length; });</script>
</body></html>"#;

pub(super) async fn site() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let body = if path.starts_with("/form") {
                    FORM.to_string()
                } else if let Some(q) = path.strip_prefix("/done?") {
                    format!("<!doctype html><title>Thanks</title><body><h1>Thanks</h1><p>got: {}</p></body>", q.replace('&', " "))
                } else {
                    "<!doctype html><title>Home</title><body><h1>Home</h1></body>".to_string()
                };
                let _ = s
                    .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes())
                    .await;
            });
        }
    });
    port
}

pub(super) fn line_ref<'a>(snapshot: &'a str, needle: &str) -> &'a str {
    snapshot
        .lines()
        .find(|l| l.contains(needle))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no line with {needle:?} in:\n{snapshot}"))
}

pub(super) fn snapshot_id(snapshot: &str) -> u64 {
    snapshot.lines().find_map(|l| l.strip_prefix("snapshot_id: ")).and_then(|v| v.trim().parse().ok()).expect("a snapshot id")
}

#[tokio::test]
#[ignore = "needs a Chrome-family browser; see the module note"]
pub(super) async fn a_real_browser_fills_a_form_by_refs_and_every_rule_holds() {
    assert!(crate::core::agentbrowser::chrome::find().is_some(), "no Chrome-family browser found");
    let dir = std::env::temp_dir().join(format!("su-live-browser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    computer::allow_loopback().store(true, Ordering::SeqCst);
    let port = site().await;
    let c = computer::get(&dir, "live-test").unwrap();

    // Read a page.
    let page = c.navigate(&format!("http://127.0.0.1:{port}/form.html"), None).await.expect("navigates");
    assert_eq!(page.title, "Sign up");
    assert!(page.text.contains("Join the newsletter"), "{}", page.text);

    // What can be acted on, by ref.
    let snap = c.snapshot().await.unwrap();
    eprintln!("{snap}");
    let id = snapshot_id(&snap);
    let name = line_ref(&snap, "\"Name\"").to_string();
    let pw = line_ref(&snap, "[password").to_string();
    let country = line_ref(&snap, "\"Country\"").to_string();
    let subscribe = line_ref(&snap, "\"Subscribe\"").to_string();
    let send = line_ref(&snap, "\"Send\"").to_string();
    assert!(snap.contains("[unchecked]"), "{snap}");

    // A password field is never typed into by the assistant.
    let refused = c.type_text(&pw, id, "hunter2", false, None).await.unwrap_err();
    assert!(refused.contains("browser_request_secret"), "{refused}");

    let typed = c.type_text(&name, id, "Ada", false, None).await.unwrap();
    assert!(typed.contains("3 characters") && !typed.contains("Ada"), "{typed}");
    // Typing again replaces rather than appends.
    c.type_text(&name, id, "Ada", false, None).await.unwrap();
    let chose = c.type_text(&country, id, "canada", false, None).await.unwrap();
    assert!(chose.contains("Chose “Canada”"), "{chose}");
    let missing = c.type_text(&country, id, "Narnia", false, None).await.unwrap_err();
    assert!(missing.contains("France"), "the options are named: {missing}");
    c.click(&subscribe, id, None).await.unwrap();

    // The secret path: asked for by ref, typed by the user, never seen by the assistant.
    c.request_secret("the account password", &pw, id, "s-live").unwrap();
    assert_eq!(c.control().secret_wanted.as_deref(), Some("the account password"));
    let chars = c.supply_secret("hunter2").await.unwrap();
    assert_eq!(chars, 7);
    assert!(c.control().secret_wanted.is_none());
    let after = c.read().await.unwrap();
    assert!(after.text.contains("length: 7"), "the value reached the field: {}", after.text);
    assert!(!after.text.contains("hunter2"));

    // While the user drives, the assistant is refused.
    c.take_control();
    assert!(c.click(&send, id, None).await.unwrap_err().contains("The user has control"));
    c.release_control();

    // Submitting moves the page on and retires the refs.
    let clicked = c.click(&send, id, None).await.unwrap();
    assert!(clicked.contains("Take a new snapshot"), "{clicked}");
    let done = c.read().await.unwrap();
    assert!(done.text.contains("name=Ada") && done.text.contains("country=CA") && done.text.contains("subscribe=on"), "{}", done.text);
    let stale = c.click(&send, id, None).await.unwrap_err();
    assert!(stale.contains("out of date"), "{stale}");

    // A picture, and a kept frame on disk.
    assert!(c.screenshot().await.unwrap().len() > 1000);
    let frame = c.keep_frame().await.expect("a frame");
    assert!(computer::frames_dir(&dir, "live-test").join(format!("{frame}.jpg")).exists());

    // The floor holds inside the browser too.
    let blocked = c.navigate("http://169.254.169.254/latest/meta-data", None).await.unwrap_err();
    assert!(blocked.starts_with("Refused.") && blocked.contains("metadata"), "{blocked}");
    computer::allow_loopback().store(false, Ordering::SeqCst);
    let loopback = c.navigate(&format!("http://127.0.0.1:{port}/"), None).await.unwrap_err();
    assert!(loopback.contains("Settings ▸ Browser"), "{loopback}");

    // Nothing typed as a secret reached the trail.
    let trail = std::fs::read_to_string(dir.join(crate::core::audit::FILE)).unwrap_or_default();
    assert!(!trail.contains("hunter2"), "{trail}");
    assert!(trail.contains("computer.secret_supplied") && trail.contains("7 characters"), "{trail}");

    // Stop keeps the profile; reset removes it.
    assert!(c.stop("test finished").await);
    assert!(!c.is_running().await);
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.reset().await.unwrap();
    assert!(!dir.join("computers/live-test/profile").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

pub(super) const BUGGY: &str = r#"<!doctype html><html><head><title>Cart</title></head><body>
<h1>Your cart</h1>
<p id="total">Total: …</p>
<script>
console.log("rendered");
console.error("Cart total is NaN");
fetch("/api/cart").then(r => r.json()).catch(() => {});
setTimeout(() => { null.total; }, 0);
</script>
</body></html>"#;

/// A staging page with a bug in it: a console error, an uncaught exception, and a request
/// that fails with a 500.
pub(super) async fn buggy_site() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, kind, body) = if path.starts_with("/api/cart") {
                    ("500 Internal Server Error", "application/json", r#"{"error":"boom"}"#.to_string())
                } else {
                    ("200 OK", "text/html", BUGGY.to_string())
                };
                let _ = s
                    .write_all(format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes())
                    .await;
            });
        }
    });
    port
}
