use super::*;

impl Computer {
    // -- the live screen ------------------------------------------------------------------

    /// Watch the screen. Frames flow while at least one watcher is attached, and the cast
    /// stops when the last one leaves.
    pub async fn watch(self: &Arc<Self>) -> (broadcast::Receiver<Screen>, Watcher) {
        let rx = self.shared.screen.subscribe();
        let first = self.shared.viewers.fetch_add(1, Ordering::SeqCst) == 0;
        if first {
            let guard = self.running.lock().await;
            if let Ok(cdp) = open_cdp(&guard) {
                start_cast(&cdp, &self.shared).await;
            }
        }
        (rx, Watcher { computer: self.clone() })
    }
}
