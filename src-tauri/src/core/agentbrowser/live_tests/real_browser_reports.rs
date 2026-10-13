use super::*;

#[tokio::test]
#[ignore = "needs a Chrome-family browser; see the module note"]
pub(super) async fn a_real_browser_reports_what_went_wrong_on_the_page() {
    assert!(crate::core::agentbrowser::chrome::find().is_some(), "no Chrome-family browser found");
    let dir = std::env::temp_dir().join(format!("su-live-console-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    computer::allow_loopback().store(true, Ordering::SeqCst);
    let port = buggy_site().await;
    let c = computer::get(&dir, "live-console").unwrap();

    c.navigate(&format!("http://127.0.0.1:{port}/cart"), None).await.expect("navigates");
    // The exception and the failed request land a moment after the load.
    let mut notes = Vec::new();
    let mut env = String::new();
    for _ in 0..20 {
        let (e, mut more) = c.console().await.expect("console");
        env = e;
        notes.append(&mut more);
        if notes.iter().any(|n| n.starts_with("uncaught exception")) && notes.iter().any(|n| n.contains("500")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    eprintln!("{env}\n{}", notes.join("\n"));
    assert!(env.contains(" on "), "the browser and the OS: {env}");
    assert!(notes.iter().any(|n| n == "console error: Cart total is NaN"), "{notes:?}");
    assert!(notes.iter().any(|n| n.starts_with("uncaught exception: TypeError") && n.contains(&format!("127.0.0.1:{port}/cart"))), "{notes:?}");
    assert!(notes.iter().any(|n| n.starts_with("network error:") && n.contains("500") && n.contains("/api/cart")), "{notes:?}");
    assert!(!notes.iter().any(|n| n.contains("rendered")), "a log is chatter: {notes:?}");

    // Read once: the next look starts empty.
    let (_, again) = c.console().await.unwrap();
    assert!(again.is_empty(), "{again:?}");

    assert!(c.stop("test finished").await);
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.reset().await.unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
