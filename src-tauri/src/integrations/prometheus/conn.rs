use super::*;

impl Conn {
    /// From the connect form's fields, or the stored connection (the same keys).
    pub fn from_fields(v: &Value) -> Result<Conn, String> {
        let get = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
        let raw = get("url");
        if raw.is_empty() {
            return Err("the Prometheus address is required — for example http://prometheus.internal:9090".into());
        }
        if !raw.contains("://") {
            return Err(format!("start the address with http:// or https:// — for example http://{raw}"));
        }
        let mut parsed = reqwest::Url::parse(&raw).map_err(|_| format!("“{raw}” isn't a web address"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none_or(str::is_empty) {
            return Err(format!("“{raw}” isn't an http or https address"));
        }
        // A login written into the address (`http://user:pass@host`) is a login: kept as one,
        // and out of the address, which ends up in links people share.
        let (in_user, in_pass) = (parsed.username().to_string(), parsed.password().unwrap_or("").to_string());
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        let (mut username, mut password) = (get("username"), get("password"));
        if username.is_empty() && password.is_empty() && !in_user.is_empty() {
            (username, password) = (decode(&in_user), decode(&in_pass));
        }
        let c = Conn { url: api_root(parsed), username, password, token: get("token"), org_id: get("org_id") };
        if !c.username.is_empty() && c.password.is_empty() {
            return Err("basic auth needs the password (or API token) as well as the username".into());
        }
        if c.username.is_empty() && !c.password.is_empty() {
            return Err("basic auth needs the username as well as the password".into());
        }
        if !c.username.is_empty() && !c.token.is_empty() {
            return Err("use basic auth or a bearer token, not both".into());
        }
        Ok(c)
    }

    /// What's kept in the keychain.
    pub fn to_stored(&self) -> String {
        json!({ "url": self.url, "username": self.username, "password": self.password, "token": self.token, "org_id": self.org_id })
            .to_string()
    }

    /// The connection the person made, or what to do about there being none.
    pub fn load() -> Result<Conn, String> {
        let stored = crate::core::secrets::get_integration_token(ID).ok_or(
            "Prometheus isn't connected. The person connects it under sidebar ▸ More ▸ Connectors ▸ Prometheus.",
        )?;
        let v: Value = serde_json::from_str(&stored).map_err(|_| "the stored Prometheus connection is unreadable — connect it again")?;
        Conn::from_fields(&v)
    }

    /// `host:port`, for labels.
    pub fn host(&self) -> String {
        reqwest::Url::parse(&self.url)
            .ok()
            .and_then(|u| Some(match u.port() { Some(p) => format!("{}:{p}", u.host_str()?), None => u.host_str()?.to_string() }))
            .unwrap_or_else(|| self.url.clone())
    }

    pub(super) async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let mut req = crate::core::egress::client("prometheus").get(format!("{}{path}", self.url)).query(query).timeout(TIMEOUT);
        if !self.username.is_empty() {
            req = req.basic_auth(&self.username, Some(&self.password));
        } else if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }
        if !self.org_id.is_empty() {
            req = req.header("X-Scope-OrgID", self.org_id.as_str());
        }
        let resp = req.send().await.map_err(|e| format!("couldn't reach Prometheus at {}: {e}", self.host()))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| format!("Prometheus's answer was cut off: {e}"))?;
        envelope(status, &text, &self.host())
    }

    /// Checks the connection, and names it: "Prometheus 3.13.1 · prometheus:9090".
    pub async fn verify(&self) -> Result<String, String> {
        match self.get("/api/v1/status/buildinfo", &[]).await {
            Ok(v) => Ok(match v["data"]["version"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                Some(version) => format!("Prometheus {version} · {}", self.host()),
                None => format!("Prometheus · {}", self.host()),
            }),
            // Not every server that speaks the query API has buildinfo; a query that
            // answers is the proof that matters.
            Err(e) if e.contains("login") => Err(e),
            Err(_) => {
                self.get("/api/v1/query", &[("query", "vector(1)".into())]).await?;
                Ok(format!("Prometheus-compatible · {}", self.host()))
            }
        }
    }

    pub async fn instant(&self, query: &str, at: f64) -> Result<(Answer, Vec<String>), String> {
        let v = self.get("/api/v1/query", &[("query", query.to_string()), ("time", format!("{at:.3}"))]).await?;
        Ok((parse_result(&v["data"])?, warnings(&v)))
    }

    pub async fn range(&self, query: &str, start: f64, end: f64, step: i64) -> Result<(Answer, Vec<String>), String> {
        let v = self
            .get(
                "/api/v1/query_range",
                &[("query", query.to_string()), ("start", format!("{start:.3}")), ("end", format!("{end:.3}")), ("step", format!("{step}s"))],
            )
            .await?;
        Ok((parse_result(&v["data"])?, warnings(&v)))
    }

    pub async fn alerts(&self) -> Result<Vec<Alert>, String> {
        let v = self.get("/api/v1/alerts", &[]).await?;
        Ok(parse_alerts(&v["data"]))
    }

    pub async fn metric_names(&self) -> Result<Vec<String>, String> {
        let v = self.get("/api/v1/label/__name__/values", &[]).await?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    pub async fn metadata(&self, metric: &str) -> Result<Meta, String> {
        let v = self.get("/api/v1/metadata", &[("metric", metric.to_string()), ("limit", "1".into())]).await?;
        let m = &v["data"][metric][0];
        Ok(Meta {
            kind: m["type"].as_str().unwrap_or("").to_string(),
            help: m["help"].as_str().unwrap_or("").to_string(),
            unit: m["unit"].as_str().unwrap_or("").to_string(),
        })
    }

    /// The label sets a metric has had in `[start, end]`, at most `MAX_SERIES` of them.
    pub async fn series(&self, metric: &str, start: f64, end: f64) -> Result<Vec<BTreeMap<String, String>>, String> {
        let v = self
            .get(
                "/api/v1/series",
                &[("match[]", metric.to_string()), ("start", format!("{start:.0}")), ("end", format!("{end:.0}")), ("limit", MAX_SERIES.to_string())],
            )
            .await?;
        Ok(v["data"].as_array().map(|a| a.iter().take(MAX_SERIES).map(labels_of).collect()).unwrap_or_default())
    }

    /// The same query in Prometheus's own graph page, ending at `end` and covering `span`
    /// seconds. `None` when the address isn't a server's root (Grafana Cloud's `/api/prom`,
    /// Mimir's `/prometheus`), which has no graph page to link to.
    pub fn graph_link(&self, query: &str, end: f64, span: i64) -> Option<String> {
        let u = reqwest::Url::parse(&self.url).ok()?;
        if !(u.path().is_empty() || u.path() == "/") {
            return None;
        }
        let end = chrono::DateTime::from_timestamp(end as i64, 0)?.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut link = reqwest::Url::parse(&format!("{}/graph", self.url)).ok()?;
        link.query_pairs_mut()
            .append_pair("g0.expr", query)
            .append_pair("g0.tab", "0")
            .append_pair("g0.range_input", &crate::core::perf::prom_duration(span))
            .append_pair("g0.end_input", &end);
        Some(link.to_string())
    }
}

/// The address the API is under: a pasted UI page (`/graph?g0.expr=…`) or API path
/// (`/api/v1/…`) means the same server, and a trailing slash is dropped.
pub(super) fn api_root(mut u: reqwest::Url) -> String {
    u.set_query(None);
    u.set_fragment(None);
    let mut path = u.path().trim_end_matches('/').to_string();
    for page in ["/graph", "/query", "/alerts", "/targets"] {
        if let Some(root) = path.strip_suffix(page) {
            path = root.to_string();
        }
    }
    if let Some(i) = path.find("/api/v1") {
        path.truncate(i);
    }
    u.set_path(&path);
    u.to_string().trim_end_matches('/').to_string()
}

/// `%40` and friends in a login written into an address.
pub(super) fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match (bytes[i], bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
