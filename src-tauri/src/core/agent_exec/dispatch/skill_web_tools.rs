use super::*;

pub(super) async fn skill_web_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    match name {
        "list_skills" | "read_skill" | "list_skill_tools" | "list_personas" | "read_persona" | "list_persona_skills"
        | "save_skill" | "save_persona" => authoring(ctx, name, args).await,
        "web_search" => {
            let q = args["query"].as_str().unwrap_or("");
            if q.trim().is_empty() {
                return "error: empty query".into();
            }
            if ctx.settings.local_only {
                return "web search is off because local-only mode is enabled.".into();
            }
            let results = if ctx.settings.searxng_url.trim().is_empty() {
                crate::core::websearch::search(q, 6).await
            } else {
                crate::core::websearch::search_searxng(&ctx.settings.searxng_url, q, 6).await
            };
            match results {
                Ok(rs) if !rs.is_empty() => {
                    // Snippets are written by whoever owns the page.
                    ctx.taint();
                    rs.iter()
                        .map(|r| format!("- {}\n  {}\n  {}", r.title, r.url, r.snippet.chars().take(220).collect::<String>()))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                Ok(_) => "No web results found.".into(),
                Err(e) => format!("web search failed: {e}"),
            }
        }
        // Ask the panel to show a page. The navigation itself happens in the GUI,
        // which owns the child webview's bounds and visibility (`BrowserTab` sizes
        // it against the panel) — so this arm validates and records the intent, and
        // the panel reacts to the tool call in the Activity feed.
        //
        // That coupling is by tool NAME, which is exactly the kind of link that
        // rots silently, so `browser_open_is_the_name_the_panel_watches_for` pins
        // it on this side and `BROWSER_OPEN_TOOL` is the single source on the other.
        "browser_open" => {
            let raw = args["url"].as_str().unwrap_or("");
            if raw.trim().is_empty() {
                return "error: provide a url".into();
            }
            let url = crate::core::browser::normalize_url(raw);
            // Same two guards `web_fetch` uses, for the same reasons — a webview
            // will happily load a `file:` or `javascript:` URL, and loopback is how
            // a page reaches this machine's own services.
            if crate::core::browser::is_blocked_scheme(&url) {
                return "error: that scheme can't be opened here (http/https only).".into();
            }
            if crate::core::webguard::is_blocked_url(&url) {
                return "error: that host is blocked (private/loopback/link-local address).".into();
            }
            // Nobody is watching a scheduled run, so claiming the page was shown
            // would be a lie — and a lie the model would then reason from.
            if ctx.unattended {
                return format!(
                    "not shown: this run is unattended, so there is no panel to show {url} in. Use web_fetch if you need its content."
                );
            }
            format!("Showing {url} in the browser panel. The user can see it now; describe what you want them to notice.")
        }
        "web_fetch" => {
            if ctx.settings.local_only {
                return "web fetch is off because local-only mode is enabled.".into();
            }
            let url = args["url"].as_str().unwrap_or("");
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return "error: provide an http(s) url".into();
            }
            if crate::core::webguard::is_blocked_url(url) {
                return "error: that host is blocked (private/loopback/link-local address).".into();
            }
            match crate::core::egress::client("web").get(url).send().await {
                Ok(resp) => {
                    // The URL that survived the redirect policy, not the one the caller
                    // typed. Everything downstream uses this.
                    let settled = resp.url().to_string();
                    match resp.text().await {
                        Ok(body) => {
                            // Optional JS rendering (off by default — see core/webrender.rs).
                            // Handed the SETTLED url so the renderer only ever contacts an
                            // address the egress guard has already vetted, and only when
                            // local-only is off, which is checked at the top of this arm.
                            //
                            // Kept only if it actually returned more than the plain fetch:
                            // that is the whole claim being tested, and a renderer that
                            // returns a shorter shell has disproved it for this page.
                            let body = match crate::core::webrender::render(&ctx.settings.web_render_cmd, &settled).await {
                                Some(rendered) if rendered.len() > body.len() => rendered,
                                _ => body,
                            };
                            // `wrap_untrusted` already tells the *model* this is data,
                            // not instructions. Tainting is the half that doesn't
                            // depend on the model choosing to believe it.
                            ctx.taint();
                            crate::core::webguard::wrap_untrusted(url, &body.chars().take(6000).collect::<String>())
                        }
                        Err(e) => format!("error reading response: {e}"),
                    }
                }
                Err(e) => format!("fetch failed: {e}"),
            }
        }
        "list_connectors" => {
            let connected: Vec<&str> = crate::integrations::descriptors::CONNECTORS
                .iter()
                .filter(|c| c.implemented && crate::core::secrets::has_integration_token(crate::integrations::token_key(c.id)))
                .map(|c| c.id)
                .collect();
            if connected.is_empty() {
                return "No apps are connected yet — the user can connect them in Integrations.".into();
            }
            format!("Connected apps: {}", connected.join(", "))
        }
        "connector_search" => {
            // Connected apps sync into the memory vault, so searching their content
            // is a scoped memory recall (category matches the app's name).
            let q = args["query"].as_str().unwrap_or("");
            if q.trim().is_empty() {
                return "error: empty query".into();
            }
            let want = args["connector"].as_str().unwrap_or("").to_lowercase();
            let mut out = String::new();
            let mut n = 0;
            for (m, _) in ctx.memory.recall_scored(q, 12) {
                if !want.is_empty() && !m.category.to_lowercase().contains(&want) && !m.tags.iter().any(|t| t.to_lowercase().contains(&want)) {
                    continue;
                }
                if m.origin == crate::core::memory::Origin::External {
                    ctx.taint();
                }
                out.push_str(&format!("### [{}] {}\n{}\n{}\n\n", m.category, m.title, m.summary, m.body.chars().take(400).collect::<String>()));
                n += 1;
                if n >= 6 {
                    break;
                }
            }
            if out.is_empty() {
                let hint = if want.is_empty() { String::new() } else { format!(" in {want}") };
                return format!("No matching content{hint}. The app may need a sync (connector_sync) or isn't connected.");
            }
            out
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
