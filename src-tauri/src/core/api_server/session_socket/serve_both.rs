use super::super::*;

/// Serve one router on both loopback listeners, and report how serving ended.
pub(in crate::core::api_server) fn serve_both(listener: Loopback, router: Router) -> (tokio::sync::watch::Sender<bool>, tokio::sync::oneshot::Receiver<Ending>) {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let (ended_tx, ended_rx) = tokio::sync::oneshot::channel();
    let Loopback { v4, v6 } = listener;
    let serve_on = |listener: tokio::net::TcpListener, router: Router, mut rx: tokio::sync::watch::Receiver<bool>| {
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = rx.changed().await;
                })
                .await
        })
    };
    let v6_task = v6.map(|v6| serve_on(v6, router.clone(), rx.clone()));
    let v4_task = serve_on(v4, router, rx.clone());
    tokio::spawn(async move {
        let outcome = v4_task.await;
        let asked = *rx.borrow();
        let ending = match outcome {
            _ if asked => Ending::Stopped,
            Ok(Ok(())) => Ending::Failed("it stopped serving without being asked to".into()),
            Ok(Err(e)) => Ending::Failed(format!("it stopped serving: {e}")),
            Err(e) if e.is_panic() => Ending::Failed(format!("it crashed: {}", crate::core::supervise::panic_text(e.into_panic()))),
            Err(_) => Ending::Failed("it was cancelled".into()),
        };
        // One side down is the port down: the restart binds both again, and a half left
        // serving would hold the port it needs.
        if let Some(t) = v6_task {
            t.abort();
        }
        let _ = ended_tx.send(ending);
    });
    (tx, ended_rx)
}
