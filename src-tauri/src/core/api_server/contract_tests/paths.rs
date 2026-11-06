/// Paths the client calls that this server does not serve, each one a known
    /// bug or a deliberately unported feature. Shrink this list; do not grow it.
    pub(super) const KNOWN_GAPS: &[&str] = &[
        // Reachable and broken — see the audit's first table.
        "/v1/settings/pdf",
        "/v1/settings/compaction",
        "/v1/settings/surfaces",
        "/v1/settings/models/add",
        "/v1/settings/models/remove",
        "/v1/settings/model-key",
        "/v1/inbox/routing",
        "/v1/inbox/routing/binding",
        "/v1/messaging/dm-route",
        "/v1/subscriptions",
        "/v1/subscriptions/remove",
        "/v1/attachments/inspect-pdf",
        "/v1/channels/recent",
        // Revealed when the sweep stopped truncating a client path at its first
        // template hole. `/v1/sessions/${id}/skills` had been passing because
        // `/v1/sessions` is served — which is how the composer's skill menu stayed a
        // 404 through two audits of this very file. These are the rest of what that
        // was hiding. The connector ones belong to the integration model this app did
        // not port; the three session ones are unbuilt features whose UI exists.
        "/v1/cloud/gallery/:x",
        "/v1/connectors/:x/accounts/:x/default",
        "/v1/connectors/:x/accounts/:x/disconnect",
        "/v1/connectors/:x/connect-managed",
        "/v1/connectors/:x/mcp-connect",
        "/v1/connectors/github/installations/:x/disconnect",
        "/v1/connectors/gmail/accounts/:x/default",
        "/v1/connectors/gmail/accounts/:x/disconnect",
        "/v1/connectors/google_calendar/accounts/:x/default",
        "/v1/connectors/google_calendar/accounts/:x/disconnect",
        "/v1/connectors/hubspot/portals/:x/default",
        "/v1/connectors/hubspot/portals/:x/disconnect",
        "/v1/connectors/slack/workspaces/:x/channels",
        "/v1/connectors/slack/workspaces/:x/directory",
        "/v1/connectors/slack/workspaces/:x/disconnect",
        "/v1/sessions/:x/artifacts/reveal",
        "/v1/sessions/:x/roots",
        "/v1/sessions/:x/unattended",
        // Helpers that take the rest of the path as an argument, so this sweep sees only
        // their prefix: `connectorUrl(name, rest)` (api/allowUser.ts, api/SlackStatus.ts) and
        // `panelBrowser(action)` (api/assistantsBrowsers.ts). Every path their callers build
        // is either served or listed here by name (checked 2026-11-07).
        "/v1/connectors/:x",
        "/v1/browser/:x",
        // Superuser's own features.
        "/v1/cloud/login",
        "/v1/cloud/logout",
        "/v1/cloud/gallery",
        "/v1/cloud/telemetry",
        "/v1/browser/state",
        "/v1/browser/screenshot",
        "/v1/browser/close",
        "/v1/connectors/github/status",
        "/v1/connectors/github/installations",
        "/v1/connectors/gmail/accounts",
        "/v1/connectors/gmail/filters",
        "/v1/connectors/google_calendar/accounts",
        "/v1/connectors/hubspot/portals",
        "/v1/connectors/hubspot/hidden-fields",
        "/v1/connectors/slack/status",
        "/v1/connectors/slack/workspaces",
        "/v1/connectors/slack/approval-owners/add",
        "/v1/connectors/slack/approval-owners/remove",
    ];

    /// Both sides are read as text. Axum exposes no way to enumerate a `Router`'s
    /// paths, and the client's are string literals — so the sweep that found these
    /// is the sweep that guards them, rather than a second approximation of it.
    pub(super) fn paths(src: &str, pattern: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (i, _) in src.match_indices(pattern) {
            // Tolerate whitespace between the marker and the string: rustfmt splits a
            // long `.route(...)` across lines, and a sweep that only matched the
            // one-line form reported every route in the split one as unserved. The
            // test must not dictate how the code is formatted.
            let rest = src[i + pattern.len()..].trim_start();
            // The marker no longer includes the opening quote (so it can tolerate a
            // newline after it), so step over the quote here instead.
            let rest = rest.strip_prefix('"').unwrap_or(rest);
            let end = rest.find(['"', '`', '?', '\'']).unwrap_or(rest.len());
            let mut p = rest[..end].to_string();
            // A template hole (`${id}`) stands for one or more path segments, so the
            // path is truncated there and marked. The mark matters: without it a
            // SHORT concrete path prefix-matches a LONGER route — `/v1/inbox/routing`
            // silently "matched" `/v1/inbox/:id/resolve` and the gap disappeared from
            // this list while remaining a gap in the product.
            // A template hole stands for ONE path segment, so `${id}` becomes `:x`
            // and whatever follows is kept. Truncating at the hole instead — which
            // this did — turned `/v1/sessions/${id}/skills` into `/v1/sessions/*`,
            // and `/v1/sessions` is served, so a missing sub-route passed the sweep.
            // That is exactly how the composer's skill menu stayed a 404 through two
            // audits.
            let mut p = {
                let mut out = String::new();
                let mut rest = p.as_str();
                while let Some(cut) = rest.find("${") {
                    out.push_str(&rest[..cut]);
                    // A hole that does not start a segment is a query string or a
                    // suffix, not a path part — `/v1/audit${q.toString()}`. The path
                    // ends there.
                    if !out.ends_with('/') {
                        rest = "";
                        break;
                    }
                    out.push_str(":x");
                    match rest[cut..].find('}') {
                        Some(end) => rest = &rest[cut + end + 1..],
                        None => {
                            rest = "";
                            break;
                        }
                    }
                }
                out.push_str(rest);
                out
            };
            p = p.trim_end_matches('/').to_string();
            let p = p;
            if p.starts_with("/v1") || p.starts_with("/ws") {
                out.push(p);
            }
        }
        out.sort();
        out.dedup();
        out
    }

    pub(super) fn served_matches(served: &[String], called: &str) -> bool {
        served.iter().any(|r| {
            let (rs, cs): (Vec<&str>, Vec<&str>) = (r.split('/').collect(), called.split('/').collect());
            // Segment for segment. A client path is now a full path with `:x` where a
            // template hole was, so there is no prefix case left — and a short
            // concrete path must never claim a longer route it would not reach.
            if cs.len() != rs.len() {
                return false;
            }
            cs.iter().zip(rs.iter()).all(|(c, r)| r.starts_with(':') || c.starts_with(':') || c == r)
        })
    }

    #[test]
    pub(super) fn the_client_calls_nothing_new_that_this_server_does_not_serve() {
        let Ok(client_src) = crate::test_support::gui_source("api.ts") else {
            // Packaged builds do not ship the client source. Nothing to check.
            return;
        };
        let server_src = &crate::test_support::module_source("core/api_server");

        let called = paths(&client_src, "${httpBase()}");
        let served = paths(server_src, ".route(");
        assert!(!called.is_empty() && !served.is_empty(), "the sweep parsed nothing — its patterns have drifted");

        let mut unexpected: Vec<String> = called
            .into_iter()
            .filter(|c| !served_matches(&served, c))
            .filter(|c| !KNOWN_GAPS.contains(&c.as_str()))
            .collect();
        unexpected.sort();

        assert!(
            unexpected.is_empty(),
            "the GUI calls paths this server does not serve, and they are not in KNOWN_GAPS:\n  {}\n\n\
             A 404 here is silent — the card renders its empty state and nobody sees a failure. \
             Either add the route, or add the path to KNOWN_GAPS with a reason.",
            unexpected.join("\n  ")
        );
    }
