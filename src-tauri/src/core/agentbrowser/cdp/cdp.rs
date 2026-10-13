use super::*;

pub const CLOSED: &str = "The browser closed.";

/// Something Chrome said without being asked.
#[derive(Debug, Clone)]
pub struct Event {
    pub method: String,
    pub params: Value,
    /// Which attached page it came from; `None` for the browser itself.
    pub session: Option<String>,
}

pub(super) type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

pub struct Cdp {
    pub(super) out: mpsc::UnboundedSender<Vec<u8>>,
    pub(super) pending: Pending,
    pub(super) next: AtomicU64,
    pub(super) events: broadcast::Sender<Event>,
    pub(super) closed: Arc<AtomicBool>,
}

impl Cdp {
    /// Start reading and writing. Generic over the byte streams so a test can stand in for
    /// Chrome with an in-memory duplex.
    pub fn start<R, W>(mut reader: R, mut writer: W) -> Arc<Cdp>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (out, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        // Large enough to ride out a burst of network events while a screencast frame is
        // being forwarded; a receiver that falls further behind skips, which for frames is
        // exactly right.
        // Deep enough that a page logging to its console in a burst can't push a dialog or a
        // navigation out of the queue before the event loop reads it.
        let (events, _) = broadcast::channel(2048);
        let closed = Arc::new(AtomicBool::new(false));

        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if writer.write_all(&bytes).await.is_err() || writer.write_all(&[0]).await.is_err() {
                    break;
                }
            }
        });

        {
            let (pending, events, closed) = (pending.clone(), events.clone(), closed.clone());
            tokio::spawn(async move {
                let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
                let mut chunk = vec![0u8; 64 * 1024];
                loop {
                    let n = match reader.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(end) = buf.iter().position(|b| *b == 0) {
                        let message: Vec<u8> = buf.drain(..=end).collect();
                        dispatch(&message[..message.len() - 1], &pending, &events);
                    }
                }
                closed.store(true, Ordering::SeqCst);
                // Everyone still waiting is told now, not at their timeout.
                if let Ok(mut p) = pending.lock() {
                    for (_, tx) in p.drain() {
                        let _ = tx.send(Err(CLOSED.into()));
                    }
                }
            });
        }

        Arc::new(Cdp { out, pending, next: AtomicU64::new(1), events, closed })
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// Send one command and wait for its answer.
    pub async fn call(&self, session: Option<&str>, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        self.call_with(session, method, params, timeout, None).await
    }

    /// The same, abandoned the moment `cancel` goes up — which is how Stop reaches a
    /// navigation that is still loading.
    pub async fn call_with(
        &self,
        session: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
        cancel: Option<&AtomicBool>,
    ) -> Result<Value, String> {
        if self.is_closed() {
            return Err(CLOSED.into());
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(s) = session {
            message["sessionId"] = json!(s);
        }
        let (tx, rx) = oneshot::channel();
        if let Ok(mut p) = self.pending.lock() {
            p.insert(id, tx);
        }
        let bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
        if self.out.send(bytes).is_err() {
            self.forget(id);
            return Err(CLOSED.into());
        }
        let deadline = tokio::time::Instant::now() + timeout;
        tokio::pin!(rx);
        loop {
            let slice = tokio::time::sleep(Duration::from_millis(100));
            tokio::select! {
                answer = &mut rx => {
                    return match answer {
                        Ok(r) => r,
                        Err(_) => Err(CLOSED.into()),
                    };
                }
                _ = slice => {
                    if cancel.is_some_and(|c| c.load(Ordering::SeqCst)) {
                        self.forget(id);
                        return Err(STOPPED.into());
                    }
                    if tokio::time::Instant::now() >= deadline {
                        self.forget(id);
                        // The method, never the params: a `Input.insertText` carrying a
                        // secret must not come back in an error message.
                        return Err(format!("Chrome didn't answer {method} within {} seconds.", timeout.as_secs()));
                    }
                }
            }
        }
    }

    pub(super) fn forget(&self, id: u64) {
        if let Ok(mut p) = self.pending.lock() {
            p.remove(&id);
        }
    }
}

/// What a cancelled call returns.
pub const STOPPED: &str = "Stopped.";

pub(super) fn dispatch(bytes: &[u8], pending: &Pending, events: &broadcast::Sender<Event>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return };
    if let Some(id) = v["id"].as_u64() {
        let answer = match v.get("error") {
            Some(e) => Err(e["message"].as_str().unwrap_or("Chrome refused that.").to_string()),
            None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
        };
        if let Some(tx) = pending.lock().ok().and_then(|mut p| p.remove(&id)) {
            let _ = tx.send(answer);
        }
        return;
    }
    if let Some(method) = v["method"].as_str() {
        let _ = events.send(Event {
            method: method.to_string(),
            params: v.get("params").cloned().unwrap_or(Value::Null),
            session: v["sessionId"].as_str().map(str::to_string),
        });
    }
}
