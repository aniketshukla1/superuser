use super::*;
use axum::extract::ws::Message as Ws;
use futures_util::StreamExt;

/// Reattaching to a session whose turn is still in flight: replay everything the turn
/// has emitted so far, then follow it live. False when the socket went away.
pub(super) async fn reattach(tx: &mut WsSink, rx: &mut WsStream, st: &crate::core::sessions::SessionStore, id: &String) -> bool {
    crate::trace!("chat", "reattach session={id} started_ms={:?}", running_since(&id));
    // `started_ms` is the point of `resumed`: the client cannot know how long this
    // turn has been going, and starting its clock at the moment of reattachment
    // makes the timer restart on every visit.
    let _ = send(tx, json!({
        "type": "turn_start",
        "data": { "resumed": true, "started_ms": running_since(&id) }
    })).await;
    if let Some(feed) = feed_get(&id) {
        let backlog: Vec<Value> = feed.0.lock().map(|b| b.clone()).unwrap_or_default();
        let mut sub = feed.1.subscribe();
        // Keep no Sender of our own — see the run loop below: an observer that holds
        // one can never be told the turn ended, and hangs here instead of sending
        // turn_done.
        drop(feed);
        for frame in backlog {
            if send(tx, frame).await.is_err() {
                return false;
            }
        }
        // Then follow live until the turn ends or this socket goes away.
        loop {
            let frame = tokio::select! {
                f = sub.recv() => match f {
                    Ok(f) => f,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                },
                incoming = rx.next() => {
                    match incoming {
                        Some(Ok(Ws::Text(t))) => {
                            let f: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
                            match f["type"].as_str() {
                                Some("interrupt") => {
                                    crate::commands::cancel_session(&id);
                                    let _ = send(tx, json!({ "type": "interrupted", "data": {} })).await;
                                }
                                // The answer to a question this run is blocked on.
                                // Handled here as well as in the live loop below
                                // because the two are different sockets: ask a
                                // question, switch chats, come back, and the socket
                                // that replays the question is this one.
                                Some("question_response") => {
                                    let a = reply_text(&f);
                                    if !a.is_empty() {
                                        crate::core::ask::answer(&id, &a);
                                    }
                                }
                                // A decision on a blocked tool call. It rides the
                                // same rendezvous as a question: the run is parked
                                // on one receiver either way, and the tool that
                                // parked it knows how to read what comes back.
                                Some("approval") => {
                                    let d = f["decision"].as_str().unwrap_or("").trim();
                                    if !d.is_empty() {
                                        crate::core::ask::answer(&id, d);
                                    }
                                }
                                // Typed while the agent is working: steer it
                                // rather than dropping the message. It goes into
                                // the transcript now, because the user did say
                                // it, and reaches the model at the next boundary.
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
                        _ => return false,
                    }
                }
            };
            if send(tx, frame).await.is_err() {
                return false;
            }
        }
    }
    // The turn finished while we were attached; tell the client so it leaves its
    // running state, then fall through to normal message handling. Outside the
    // `if let` on purpose: when the turn ends between the is_running check and
    // feed_get there is no feed to follow, and a client that got `turn_start`
    // would otherwise be left spinning with no turn_done ever coming.
    if let Some(sess) = st.get(&id) {
        if let Some(last) = sess.messages.iter().rev().find(|m| m.role == "assistant") {
            let _ = send(tx, json!({ "type": "assistant_message", "data": { "text": last.content, "persona": last.persona } })).await;
        }
    }
    let _ = send(tx, json!({ "type": "turn_done", "data": { "artifacts": [], "todos": [] } })).await;
    true
}
