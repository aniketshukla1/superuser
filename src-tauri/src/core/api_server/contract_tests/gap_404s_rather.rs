use super::*;

    /// A gap has to be a 404. Landing on the WRONG handler is the worse failure, and
    /// it is invisible from both sides: the client gets a 200 with a shape it will
    /// try to read, and the server logs a successful request.
    ///
    /// `/v1/personas/install` was exactly this — swallowed by `/v1/personas/:id` and
    /// answered as an assistant called "install" — and neither half of the ratchet
    /// above could see it, because both compare strings and a string comparison
    /// cannot know that `:id` matches the word "install". This one asks the real
    /// router.
    #[tokio::test]
    pub(super) async fn a_gap_404s_rather_than_landing_on_the_wrong_handler() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use axum::routing::get;
        use axum::Router;
        use tower::ServiceExt;

        let server_src = &crate::test_support::module_source("core/api_server");
        let served = paths(server_src, ".route(");
        assert!(!served.is_empty(), "the sweep parsed nothing — its patterns have drifted");

        // Rebuild the shape of the real table. Parameter NAMES are normalised because
        // matchit refuses two routes that use different names at the same position,
        // and only the shape matters here.
        async fn hit() -> &'static str {
            "hit"
        }
        let mut shapes: Vec<String> = served
            .iter()
            .filter(|p| p.starts_with("/v1"))
            .map(|p| {
                p.split('/')
                    .map(|seg| if seg.starts_with(':') { ":p" } else { seg })
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect();
        shapes.sort();
        shapes.dedup();
        let mut app = Router::new();
        for shape in &shapes {
            app = app.route(
                shape,
                get(hit).post(hit).patch(hit).delete(hit).put(hit),
            );
        }

        // Every known gap, as a concrete URL a user could actually cause.
        let mut misrouted: Vec<String> = Vec::new();
        for gap in KNOWN_GAPS {
            let url = gap.split('/').map(|s| if s == ":x" { "sample" } else { s }).collect::<Vec<_>>().join("/");
            for method in ["GET", "POST"] {
                let req = Request::builder().method(method).uri(&url).body(Body::empty()).unwrap();
                let res = app.clone().oneshot(req).await.unwrap();
                if res.status() != StatusCode::NOT_FOUND {
                    misrouted.push(format!("{method} {url} → {} (gap: {gap})", res.status()));
                }
            }
        }

        assert!(
            misrouted.is_empty(),
            "these unbuilt paths reach a handler instead of 404ing, so the client gets a \
             success it will try to read:\n  {}\n\n\
             A wildcard segment is swallowing them. Register the concrete route (it wins over \
             the wildcard), or narrow the wildcard.",
            misrouted.join("\n  ")
        );
    }

    #[test]
    pub(super) fn every_known_gap_is_still_a_gap() {
        // The other half of the ratchet: once a route is added, its entry has to go,
        // or the list rots into a permanent excuse.
        let server_src = &crate::test_support::module_source("core/api_server");
        let served = paths(server_src, ".route(");
        let stale: Vec<&str> = KNOWN_GAPS.iter().copied().filter(|g| served_matches(&served, g)).collect();
        assert!(
            stale.is_empty(),
            "these are served now — delete them from KNOWN_GAPS:\n  {}",
            stale.join("\n  ")
        );
    }
