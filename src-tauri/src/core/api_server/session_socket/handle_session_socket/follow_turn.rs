use super::*;
use axum::extract::ws::Message as Ws;
use futures_util::StreamExt;

/// Follows a running turn's feed on this socket until it ends or the socket closes, and
/// returns how the turn ended.
pub(super) async fn follow_turn(tx: &mut WsSink, rx: &mut WsStream, st: &crate::core::sessions::SessionStore, id: &String, feed: RunFeed, run: tauri::async_runtime::JoinHandle<Result<crate::commands::AgentRunOut, String>>) -> Result<crate::commands::AgentRunOut, String> {
    // Follow the feed (so this socket sees the same frames as any other
    // observer) while still reading the socket, so an interrupt lands
    // while the turn is in flight rather than after it finishes.
    //
    // Subscribe, then drop this socket's OWN handle to the feed. Holding a
    // broadcast Sender here kept the channel open forever, so "every sender
    // is gone" — the signal that the turn ended — could never arrive: the
    // loop parked on recv() after the answer was already computed and
    // persisted, and the client, still waiting for assistant_message /
    // turn_done, sat on its last tool step. Switching chats closed the
    // socket and the transcript re-read showed the answer instantly, which
    // is exactly what a stuck turn looked like.
    let mut sub = feed.1.subscribe();
    drop(feed);
    // Watch the run itself too, so a turn that panics before it can close
    // its feed (leaving the map's Sender behind) still ends this loop.
    let mut run = Box::pin(run);
    let mut result: Option<Result<crate::commands::AgentRunOut, String>> = None;
    loop {
        let frame = tokio::select! {
            // Polled in order: an interrupt outranks a frame (Stop has to
            // land fast), and the run's completion is checked only once the
            // feed has nothing left to forward — otherwise finishing the turn
            // could cut ahead of the last steps still queued for this socket.
            biased;
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Ws::Text(t))) => {
                        let f: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
                        match f["type"].as_str() {
                            Some("interrupt") => {
                                crate::commands::cancel_session(&id);
                                let _ = send(tx, json!({ "type": "interrupted", "data": {} })).await;
                            }
                            // The answer the tool is parked on. Same
                            // reasoning as the reattach loop above: both
                            // sockets have to accept it.
                            Some("question_response") => {
                                let a = reply_text(&f);
                                if !a.is_empty() {
                                    crate::core::ask::answer(&id, &a);
                                }
                            }
                            // See the reattach loop above: both sockets
                            // have to accept a decision, or approving
                            // works only after switching chats and back.
                            Some("approval") => {
                                let d = f["decision"].as_str().unwrap_or("").trim();
                                if !d.is_empty() {
                                    crate::core::ask::answer(&id, d);
                                }
                            }
                            // The common way to steer: typed in the same
                            // window that started the turn. (The reattach
                            // loop above handles the other one — both
                            // paths have to accept it, or steering works
                            // only after switching chats and back.)
                            Some("user_message") => {
                                let text = f["text"].as_str().unwrap_or("").trim().to_string();
                                if !text.is_empty() {
                                    queue_steer(&id, &text);
                                    st.append(&id, crate::core::sessions::Message::new("user", text.clone()));
                                    let _ = send(tx, json!({ "type": "steered", "data": { "text": text } })).await;
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    // Socket closed: stop forwarding. The run continues,
                    // keeps filling its feed, and persists its result.
                    _ => break,
                }
            }
            f = sub.recv() => match f {
                Ok(f) => f,
                // Lagged: we fell behind the buffer. Keep going rather
                // than dropping the user out of a live turn.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            joined = &mut run, if result.is_none() => {
                result = Some(joined.unwrap_or_else(|e| Err(format!("run failed: {e}"))));
                // The forwarder drains its channel before the run's tail
                // runs, so everything it produced is already buffered here:
                // flush it, then finish the turn.
                loop {
                    match sub.try_recv() {
                        Ok(f) => {
                            if send(tx, f).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
                break;
            }
        };
        if send(tx, frame).await.is_err() {
            break;
        }
    }
    match result {
        Some(r) => r,
        None => run.await.unwrap_or_else(|e| Err(format!("run failed: {e}"))),
    }
}
