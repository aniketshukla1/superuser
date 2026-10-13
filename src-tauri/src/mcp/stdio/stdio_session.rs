use super::*;

pub(super) const PROTO: &str = "2025-06-18";

/// Handshake and listing should be quick; a call may do real work.
pub(super) const OPEN_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) const CALL_TIMEOUT: Duration = Duration::from_secs(120);

pub struct StdioSession {
    pub(super) child: Child,
    pub(super) stdin: ChildStdin,
    pub(super) out: BufReader<ChildStdout>,
    pub(super) seq: i64,
    pub(super) server_name: Option<String>,
}

/// Quote one argv element for `sh -lc`. Single quotes with the standard
/// `'\''` escape, so a path with a space or an apostrophe survives the shell.
pub(super) fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// The command line a config describes: `command` plus optional `args`.
pub fn command_line(config: &Value) -> Option<String> {
    let cmd = config.get("command").and_then(|v| v.as_str())?.trim();
    if cmd.is_empty() {
        return None;
    }
    let mut line = shell_quote(cmd);
    if let Some(args) = config.get("args").and_then(|v| v.as_array()) {
        for a in args {
            if let Some(s) = a.as_str() {
                line.push(' ');
                line.push_str(&shell_quote(s));
            }
        }
    }
    Some(line)
}

impl StdioSession {
    /// Launch the server and complete the MCP handshake.
    pub async fn open(config: &Value) -> Result<StdioSession, String> {
        let line = command_line(config).ok_or("this server has no `command` to run")?;

        // Through a login shell, for the same reason the LSP bridge does it: `npx`
        // and `uvx` live on a PATH set by the user's shell profile, and an app
        // launched from Finder does not inherit it. Spawned directly, the common
        // case would fail with "No such file or directory" on a binary the user can
        // plainly run in their terminal.
        #[cfg(windows)]
        let mut c = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&line);
            c
        };
        #[cfg(not(windows))]
        let mut c = {
            let mut c = Command::new("sh");
            c.arg("-lc").arg(format!("exec {line}"));
            c
        };

        if let Some(env) = config.get("env").and_then(|v| v.as_object()) {
            for (k, v) in env {
                if let Some(s) = v.as_str() {
                    c.env(k, s);
                }
            }
        }
        if let Some(cwd) = config.get("cwd").and_then(|v| v.as_str()) {
            c.current_dir(cwd);
        }

        // stderr is where these servers write their logs and their startup errors.
        // Inheriting it would scatter another program's noise through ours, and a
        // pipe nobody drains fills and blocks the child, so it goes to null.
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("couldn't start `{line}`: {e}"))?;

        let stdin = child.stdin.take().ok_or("no stdin on the spawned server")?;
        let stdout = child.stdout.take().ok_or("no stdout on the spawned server")?;
        let mut s = StdioSession { child, stdin, out: BufReader::new(stdout), seq: 1, server_name: None };

        let init = s
            .rpc(
                json!({
                    "jsonrpc":"2.0","id":1,"method":"initialize",
                    "params":{"protocolVersion":PROTO,"capabilities":{},"clientInfo":{"name":"Superuser","version":"0.1.0"}}
                }),
                OPEN_TIMEOUT,
            )
            .await?;
        s.server_name = init["serverInfo"]["name"].as_str().map(String::from);
        // Notification: no id, so nothing to wait for.
        let _ = s.notify(json!({ "jsonrpc":"2.0","method":"notifications/initialized" })).await;
        Ok(s)
    }

    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Value>, String> {
        self.seq += 1;
        let r = self
            .rpc(json!({ "jsonrpc":"2.0","id":self.seq,"method":"tools/list" }), OPEN_TIMEOUT)
            .await?;
        Ok(r["tools"].as_array().cloned().unwrap_or_default())
    }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, String> {
        self.seq += 1;
        self.rpc(
            json!({
                "jsonrpc":"2.0","id":self.seq,"method":"tools/call",
                "params":{ "name": name, "arguments": args }
            }),
            CALL_TIMEOUT,
        )
        .await
    }

    pub(super) async fn notify(&mut self, body: Value) -> Result<(), String> {
        let line = format!("{body}\n");
        self.stdin.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
        self.stdin.flush().await.map_err(|e| e.to_string())
    }

    /// Send one request and read until the reply with the matching id arrives.
    ///
    /// Servers interleave notifications and log lines with replies, so matching on
    /// id — rather than taking the next line — is what keeps a chatty server from
    /// desynchronising the stream.
    pub(super) async fn rpc(&mut self, body: Value, timeout: Duration) -> Result<Value, String> {
        let want = body["id"].clone();
        self.notify(body).await?;

        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let mut line = String::new();
            let read = tokio::time::timeout_at(deadline, self.out.read_line(&mut line)).await;
            let n = match read {
                Err(_) => return Err("the MCP server didn't answer in time".into()),
                Ok(Err(e)) => return Err(format!("MCP server read failed: {e}")),
                Ok(Ok(n)) => n,
            };
            if n == 0 {
                // EOF: the process exited. Its own exit status says more than
                // "connection closed" would.
                let status = self.child.try_wait().ok().flatten();
                return Err(match status {
                    Some(s) => format!("the MCP server exited ({s}) without answering"),
                    None => "the MCP server closed its output without answering".into(),
                });
            }
            let Ok(v) = serde_json::from_str::<Value>(line.trim()) else {
                // Not JSON: a server that printed a banner to stdout. Skip it rather
                // than fail — the reply may still be on its way.
                continue;
            };
            if v.get("id") != Some(&want) {
                continue;
            }
            if let Some(err) = v.get("error") {
                return Err(format!("MCP error: {}", err["message"].as_str().unwrap_or("unknown")));
            }
            return Ok(v["result"].clone());
        }
    }
}
