use super::*;

pub(super) async fn run_under(
    http: &crate::core::egress::Client,
    endpoint: &str,
    token: Option<&str>,
    input: &Value,
    floor: fn(&str) -> Option<String>,
    on_event: &mut (dyn FnMut(&Value) + Send),
) -> Result<Answer, String> {
    if let Some(problem) = floor(endpoint) {
        return Err(format!("Refused. {problem}"));
    }
    let work = async {
        let mut url = endpoint.to_string();
        let mut carry_token = token;
        let mut hops = 0usize;
        let mut reply = loop {
            let mut request = http
                .post(&url)
                .header("content-type", "application/json")
                .header("accept", "text/event-stream")
                .json(input);
            if let Some(t) = carry_token {
                request = request.header("authorization", format!("Bearer {t}"));
            }
            let reply = tokio::time::timeout(CONNECT_TIMEOUT, request.send())
                .await
                .map_err(|_| "that agent didn't answer in time".to_string())?
                .map_err(|e| format!("couldn't reach that agent: {e}"))?;
            if !reply.status().is_redirection() {
                break reply;
            }
            hops += 1;
            if hops > MAX_HOPS {
                return Err("that agent redirected too many times".to_string());
            }
            let location = reply
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or("that agent redirected without saying where")?
                .to_string();
            let next = reqwest::Url::parse(&url)
                .and_then(|base| base.join(&location))
                .map(|u| u.to_string())
                .map_err(|_| "that agent redirected to an address that doesn't parse".to_string())?;
            if let Some(problem) = floor(&next) {
                return Err(format!("Refused. That agent redirected somewhere this app won't follow: {problem}"));
            }
            // A different host is a different party: whatever we were sent is not for them.
            if crate::core::boundaries::host_of(&next) != crate::core::boundaries::host_of(&url) {
                carry_token = None;
            }
            url = next;
        };
        let status = reply.status();
        if !status.is_success() {
            let body = reply.text().await.unwrap_or_default();
            return Err(format!("that agent answered {}: {}", status.as_u16(), body.chars().take(200).collect::<String>()));
        }
        let kind = reply.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        if !kind.starts_with("text/event-stream") {
            return Err(format!(
                "that address answered, but not with an event stream ({}) — is it an AG-UI agent's run endpoint?",
                if kind.is_empty() { "no content type" } else { kind.as_str() }
            ));
        }
        let mut reader = Reader::default();
        let mut buffer = String::new();
        'read: while let Some(chunk) = reply.chunk().await.map_err(|e| format!("the connection to that agent broke: {e}"))? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            if buffer.contains('\r') {
                buffer = buffer.replace("\r\n", "\n");
            }
            while let Some(at) = buffer.find("\n\n") {
                let block: String = buffer.drain(..at + 2).collect();
                if let Some(event) = events_in(&block) {
                    on_event(&event);
                    if !reader.feed(&event) {
                        break 'read;
                    }
                }
            }
            if buffer.len() > MAX_ANSWER_CHARS * 4 {
                // One event that never ends is the same fault as an answer that never does.
                break;
            }
        }
        Ok(reader.finish())
    };
    tokio::time::timeout(RUN_TIMEOUT, work).await.map_err(|_| "that agent ran past five minutes and was stopped".to_string())?
}

/// What "Test connection" reports: whether it answered, and whether what came back was
/// AG-UI at all.
pub async fn test(http: &crate::core::egress::Client, endpoint: &str, token: Option<&str>) -> Result<String, String> {
    let input = input_for("test-thread", "test-run", &[], "Say hello in five words or fewer.");
    let answer = run(http, endpoint, token, &input, &mut |_| {}).await?;
    judge_test(&answer)
}

pub(super) fn judge_test(answer: &Answer) -> Result<String, String> {
    match (&answer.error, answer.text.trim()) {
        (Some(e), _) => Err(format!("It answered, and the run ended with: {e}")),
        (None, "") => Err("It answered, but the run carried no reply — check it is an AG-UI agent.".into()),
        (None, text) => Ok(format!("It answered: “{}”", text.chars().take(160).collect::<String>())),
    }
}
