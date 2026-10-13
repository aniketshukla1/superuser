use super::*;

impl Speaker {
    pub fn queue_len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Claim the drain job. Returns false when another worker already holds it, so
    /// two calls in the same breath produce one voice rather than two talking over
    /// each other.
    pub fn claim_drain(&self) -> bool {
        !self.draining.swap(true, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn release_drain(&self) {
        self.draining.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Clear the flag at the start of a new answer.
    pub fn begin(&self) {
        self.cancel.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn speaking(&self) -> bool {
        let mut cur = self.current.lock().unwrap();
        match cur.as_mut() {
            None => false,
            Some(c) => match c.try_wait() {
                Ok(Some(_)) => {
                    *cur = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            },
        }
    }
}
