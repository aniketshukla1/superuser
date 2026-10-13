use super::super::*;

/// A `reqwest::Client` that knows what it is for.
///
/// Mirrors the small slice of reqwest's builder API the app actually uses. The
/// point of the newtype is not the methods — it's that `send` is ours, so there is
/// no way to hold one of these and reach the network unobserved.
#[derive(Clone)]
pub struct Client {
    pub(in crate::core::egress) inner: reqwest::Client,
    pub(in crate::core::egress) purpose: String,
}

/// May a redirect chain that started at `origin_host` be followed to `next_host`?
///
/// The guards in `webguard` inspect the URL the caller supplied and nothing else,
/// but reqwest follows up to ten redirects by default and no one re-checks the
/// hops. So `web_fetch("https://innocent.example")` could be bounced onto
/// `169.254.169.254` and the SSRF check would never have seen the address it
/// actually fetched. Every host-table fix is decoration until this closes.
///
/// The rule is about crossing the boundary, not about the destination: a request
/// that began on this machine may keep redirecting around it (Ollama, the local
/// API, and OAuth's loopback callback all do), but a chain that began off-machine
/// may never land on it.
pub(crate) fn redirect_allowed(origin_host: &str, next_host: &str) -> bool {
    is_local(origin_host) || !is_local(next_host)
}

/// Build a client for a named purpose. The purpose is what makes the log worth
/// reading: "api.github.com, 41 requests" is a fact, "connector sync" is an answer.
pub fn client(purpose: &str) -> Client {
    let inner = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            // `previous()` is the chain so far, oldest first; its head is the URL
            // the caller actually asked for and the one the guards vetted.
            let origin = attempt.previous().first().and_then(|u| u.host_str()).unwrap_or("").to_string();
            let next = attempt.url().host_str().unwrap_or("").to_string();
            if !redirect_allowed(&origin, &next) {
                return attempt.error(format!(
                    "refused to follow a redirect from {origin} to {next} — a public address may not \
                     redirect onto this machine's private network"
                ));
            }
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            attempt.follow()
        }))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    Client { inner, purpose: purpose.to_string() }
}

/// A client that follows no redirects at all, for a caller that must check every hop
/// against a stricter floor than [`redirect_allowed`] — a remote agent's address has to be
/// https and nameable on each hop, not merely off this machine. The redirect comes back as
/// the response, and the caller decides.
pub fn client_no_follow(purpose: &str) -> Client {
    let inner = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    Client { inner, purpose: purpose.to_string() }
}

impl Client {
    pub fn get<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.get(url))
    }
    pub fn post<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.post(url))
    }
    // The verbs below have no call site yet. They are here anyway: the wrapper's
    // job is to be a complete enough stand-in that nobody is ever tempted around
    // it, and a missing `delete` is exactly the sort of gap that ends with someone
    // reaching for `reqwest::Client::new()`.
    #[allow(dead_code)]
    pub fn put<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.put(url))
    }
    #[allow(dead_code)]
    pub fn patch<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.patch(url))
    }
    #[allow(dead_code)]
    pub fn delete<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.delete(url))
    }
    #[allow(dead_code)]
    pub fn head<U: reqwest::IntoUrl>(&self, url: U) -> RequestBuilder {
        self.wrap(self.inner.head(url))
    }

    pub(in crate::core::egress) fn wrap(&self, inner: reqwest::RequestBuilder) -> RequestBuilder {
        RequestBuilder { inner, client: self.inner.clone(), purpose: self.purpose.clone() }
    }
}

/// Forwards to reqwest's builder, then routes `send` through the spine.
pub struct RequestBuilder {
    pub(in crate::core::egress) inner: reqwest::RequestBuilder,
    pub(in crate::core::egress) client: reqwest::Client,
    pub(in crate::core::egress) purpose: String,
}

impl RequestBuilder {
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        reqwest::header::HeaderName: TryFrom<K>,
        <reqwest::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        reqwest::header::HeaderValue: TryFrom<V>,
        <reqwest::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self { inner: self.inner.header(key, value), ..self }
    }

    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        Self { inner: self.inner.json(json), ..self }
    }

    pub fn form<T: Serialize + ?Sized>(self, form: &T) -> Self {
        Self { inner: self.inner.form(form), ..self }
    }

    pub fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        Self { inner: self.inner.query(query), ..self }
    }

    pub fn body<T: Into<reqwest::Body>>(self, body: T) -> Self {
        Self { inner: self.inner.body(body), ..self }
    }

    forward!(timeout, timeout: std::time::Duration);

    pub fn bearer_auth<T: std::fmt::Display>(self, token: T) -> Self {
        Self { inner: self.inner.bearer_auth(token), ..self }
    }

    pub fn basic_auth<U: std::fmt::Display, P: std::fmt::Display>(self, username: U, password: Option<P>) -> Self {
        Self { inner: self.inner.basic_auth(username, password), ..self }
    }

    /// Send it — or don't, and say why.
    ///
    /// The error a refusal returns is deliberately a plain message rather than a
    /// `reqwest::Error`: there is no transport failure to report, because nothing
    /// was attempted. Callers that want the network's own error type keep getting
    /// it for everything that actually goes out.
    pub async fn send(self) -> Result<reqwest::Response, EgressError> {
        let req = self.inner.build().map_err(EgressError::Http)?;
        let url = req.url().to_string();
        let (host, path) = split_url(&url).unwrap_or_else(|| (url.clone(), String::new()));
        let method = req.method().to_string();
        let sent = req.body().and_then(|b| b.as_bytes()).map(|b| b.len() as u64).unwrap_or(0);

        // Loopback and private addresses never left the machine, so local-only has
        // no opinion about them and the log has nothing to say.
        if is_local(&host) {
            return self.client.execute(req).await.map_err(EgressError::Http);
        }

        if local_only() {
            let error = format!(
                "Local-only mode is on, so nothing may leave this machine — {host} was not contacted. \
                 Turn it off in Settings to allow this."
            );
            record(Egress {
                at: now(),
                purpose: self.purpose,
                method,
                host,
                path,
                sent,
                status: None,
                error: Some(error.clone()),
                blocked: true,
            });
            return Err(EgressError::LocalOnly(error));
        }

        let out = self.client.execute(req).await;
        record(Egress {
            at: now(),
            purpose: self.purpose,
            method,
            host,
            path,
            sent,
            status: out.as_ref().ok().map(|r| r.status().as_u16()),
            error: out.as_ref().err().map(|e| e.to_string().chars().take(200).collect()),
            blocked: false,
        });
        out.map_err(EgressError::Http)
    }
}
