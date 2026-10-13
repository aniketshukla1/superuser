use super::*;

    /// This turn's audit rows, oldest first. Empty on any failure — "not measured".
    pub(super) async fn trail(http: &crate::core::egress::Client, tok: &str, sid: &str, since: f64) -> Vec<TrailRow> {
        // Session ids here are alphanumerics and hyphens (`sanitise`), so they need no
        // escaping in a query string.
        let url = api(&format!("/v1/audit?session_id={sid}&since={since}&limit=2000"));
        let Ok(resp) = http.get(url).header("x-superuser-token", tok).send().await else { return Vec::new() };
        if !resp.status().is_success() {
            return Vec::new();
        }
        let Ok(v) = resp.json::<Value>().await else { return Vec::new() };
        let mut rows: Vec<TrailRow> = v["events"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|e| {
                        Some(TrailRow {
                            // An older build answers without `event`; its rows say
                            // nothing a check here reads.
                            event: e["event"].as_str()?.to_string(),
                            tool: e["subject"]["tool"].as_str().unwrap_or("").to_string(),
                            at: e["at"].as_f64().unwrap_or(0.0),
                            decision: serde_json::from_value(e["decision"].clone()).ok(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        rows.reverse(); // the route answers newest first
        rows
    }
