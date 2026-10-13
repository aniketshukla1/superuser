use super::*;

    /// Install this task's assistants and say who may be asked.
    ///
    /// The files go straight into the app's `personas/` directory rather than through the
    /// create endpoint: a task's assistant is a whole manifest — a standing role with a
    /// fact in it — and the form that endpoint takes cannot express one. The registry is
    /// read per request, so they are live on the next turn without a restart.
    pub(super) async fn seed_personas(
        http: &crate::core::egress::Client,
        tok: &str,
        settings: &Value,
        task: &Task,
        marker: &str,
    ) -> Result<(), String> {
        let Some(vault) = scratch_vault(settings) else {
            return Err("assistants can only be installed on a scratch instance".into());
        };
        let dir = vault.parent().ok_or("no data dir")?.join("personas");
        std::fs::create_dir_all(&dir).map_err(|e| format!("personas dir: {e}"))?;
        // Cleared first, so one task's colleagues are not still in the room for the next.
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().extension().is_some_and(|x| x == "md") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        for (name, manifest) in task.personas {
            std::fs::write(dir.join(name), manifest.replace(MARKER_SLOT, marker))
                .map_err(|e| format!("assistant {name}: {e}"))?;
            // A user-installed assistant lands disabled, pending the person's consent. On a
            // scratch instance the driver is the person.
            let id = name.trim_end_matches(".md");
            post_ok(http, tok, &format!("/v1/personas/{id}"), json!({ "enabled": true })).await?;
        }
        let asks: Vec<&str> = task.asks.to_vec();
        post_ok(http, tok, "/v1/personas/assistant", json!({ "asks": asks })).await?;
        Ok(())
    }

    /// POST, and insist the app said `ok`.
    /// A GET the driver reads, or `None` when it can't be read.
    pub(super) async fn get_value(http: &crate::core::egress::Client, tok: &str, path: &str) -> Option<Value> {
        let r = http.get(api(path)).header("x-superuser-token", tok).send().await.ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.json().await.ok()
    }

    pub(super) async fn post_ok(http: &crate::core::egress::Client, tok: &str, path: &str, body: Value) -> Result<Value, String> {
        let v: Value = http
            .post(api(path))
            .header("x-superuser-token", tok)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("{path}: {e}"))?
            .json()
            .await
            .map_err(|e| format!("{path}: {e}"))?;
        if v["ok"] != json!(true) {
            return Err(format!("{path} refused: {v}"));
        }
        Ok(v)
    }

    /// A browser for the assistant, fresh for this task, and this conversation's "Always
    /// allow" on the fixture site — its host and port, as a person's grant would be. Signed out
    /// first, so nothing one task left open or signed in to is waiting for the next.
    pub(super) async fn prepare_browser(http: &crate::core::egress::Client, tok: &str, sid: &str, base: &str) -> Result<(), String> {
        post_ok(http, tok, "/v1/settings/computer", json!({ "browser_enabled": true, "browser_allow_loopback": true })).await?;
        post_ok(http, tok, "/v1/personas/assistant", json!({ "browser": true })).await?;
        post_ok(http, tok, "/v1/computers/assistant/reset", json!({})).await?;
        let grant = format!("browser@{}", crate::core::agent_exec::site_of(base));
        post_ok(http, tok, &format!("/v1/sessions/{sid}/grants"), json!({ "grant": grant })).await?;
        Ok(())
    }

    /// A browser task's pages, served on a loopback port for as long as this is held.
    pub(super) struct Site {
        pub(super) base: String,
        pub(super) server: tokio::task::JoinHandle<()>,
    }

    impl Drop for Site {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    /// One request at a time per connection, headers only — a posted form's body is the
    /// page's business and no page reads it — and `Connection: close` after each.
    pub(super) async fn serve_site(pages: &'static [Page], marker: String) -> std::io::Result<Site> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
        let server = tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let marker = marker.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 16 * 1024];
                    let mut got = 0;
                    while got < buf.len() {
                        match sock.read(&mut buf[got..]).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                got += n;
                                if buf[..got].windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                            }
                        }
                    }
                    let head = String::from_utf8_lossy(&buf[..got]).to_string();
                    let target = head.split_whitespace().nth(1).unwrap_or("/");
                    let (status, body) = match site_response(pages, target, &marker) {
                        Some(html) => ("200 OK", html),
                        None => ("404 Not Found", "<!doctype html><title>Not found</title><h1>Not found</h1>".to_string()),
                    };
                    let reply = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(reply.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        Ok(Site { base, server })
    }

    /// Put this task's boundaries and standing instructions in place — or clear them.
    pub(super) async fn apply_config(http: &crate::core::egress::Client, tok: &str, task: &Task, marker: &str) -> Result<(), String> {
        let sent = match task.boundaries {
            Some(text) => {
                let body: Value = serde_json::from_str(text).map_err(|e| format!("{}: its boundaries aren't JSON: {e}", task.id))?;
                http.post(api("/v1/boundaries")).header("x-superuser-token", tok).json(&body).send().await
            }
            None => http.post(api("/v1/boundaries/reset")).header("x-superuser-token", tok).send().await,
        };
        let v: Value = sent.map_err(|e| format!("boundaries: {e}"))?.json().await.map_err(|e| format!("boundaries: {e}"))?;
        if v["ok"] != json!(true) {
            // An invalid rule in a task is the task's bug, and it says which rule.
            return Err(format!("{}: boundaries not applied: {v}", task.id));
        }
        let v: Value = http
            .post(api("/v1/settings/standing-instructions"))
            .header("x-superuser-token", tok)
            .json(&json!({ "text": task.standing.replace(MARKER_SLOT, marker) }))
            .send()
            .await
            .map_err(|e| format!("standing instructions: {e}"))?
            .json()
            .await
            .map_err(|e| format!("standing instructions: {e}"))?;
        if v["ok"] != json!(true) {
            return Err(format!("{}: standing instructions not applied: {v}", task.id));
        }
        Ok(())
    }
