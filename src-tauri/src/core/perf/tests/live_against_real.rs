use super::*;

    /// Every tool against a real Prometheus, read-only:
    ///
    ///     PROM_URL=http://localhost:9090 cargo test --lib core::perf::tests::live -- --ignored --nocapture
    ///
    /// With `PROM_CHANGE_AT` (RFC 3339), also compares the query handlers' latency either side
    /// of it — run some heavy queries after that moment to give it something to find.
    #[tokio::test]
    #[ignore = "needs a Prometheus; see the note"]
    pub(super) async fn live_against_a_real_prometheus() {
        use serde_json::json;
        let url = std::env::var("PROM_URL").unwrap_or_else(|_| "http://localhost:9090".into());
        let conn = Conn::from_fields(&json!({ "url": url })).unwrap();
        println!("connected: {}\n", conn.verify().await.unwrap());
        let dir = std::env::temp_dir().join(format!("su-perf-live-{}", crate::core::inbox::short_id()));
        let now = Utc::now();
        for (name, args) in [
            ("metrics_find", json!({ "text": "http request duration" })),
            ("metrics_find", json!({ "text": "prometheus_http_request_duration_seconds_bucket" })),
            ("metrics_query", json!({ "query": "sum by (handler) (rate(prometheus_http_requests_total[5m]))" })),
            ("metrics_range", json!({ "query": "rate(process_cpu_seconds_total[1m])", "since": "30m" })),
            ("metrics_alerts", json!({})),
        ] {
            let out = run(&conn, name, &args, "", &dir, now).await.unwrap_or_else(|e| panic!("{name}: {e}"));
            println!("=== {name} {args}\n{}\n", out.text);
            for f in &out.files {
                assert!(dir.join(f).exists(), "{f}");
            }
        }
        assert!(run(&conn, "metrics_query", &json!({ "query": "sum(" }), "", &dir, now).await.unwrap_err().contains("didn't parse"));
        if let Ok(change) = std::env::var("PROM_CHANGE_AT") {
            // The percentile, and the average: a histogram's percentile moves only between its
            // buckets, so a change inside one bucket shows in the average alone.
            for q in [
                "histogram_quantile(0.9, sum by (le, handler) (rate(prometheus_http_request_duration_seconds_bucket[1m])))",
                "sum by (handler) (rate(prometheus_http_request_duration_seconds_sum[1m])) / sum by (handler) (rate(prometheus_http_request_duration_seconds_count[1m]))",
            ] {
                let out = run(&conn, "metrics_compare", &json!({ "query": q, "change_at": change, "window": "5m" }), "", &dir, now).await.unwrap();
                println!("=== metrics_compare\n{}\n", out.text);
                assert!(out.text.contains("/api/v1/query"), "{}", out.text);
            }
        }
        let signals = "Query p90: histogram_quantile(0.9, sum by (le) (rate(prometheus_http_request_duration_seconds_bucket{handler=\"/api/v1/query\"}[1m])))\nCPU: rate(process_cpu_seconds_total[1m])\nBroken: sum(";
        let out = run(&conn, "perf_report", &json!({ "window": "5m" }), signals, &dir, now).await.unwrap();
        println!("=== perf_report\n{}", out.text);
        assert!(out.text.contains("Broken — the query didn't parse"), "{}", out.text);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn alerts_read_as_a_list_that_changes_nothing() {
        let now = at("2026-09-24T10:00:00Z");
        assert_eq!(alerts_text(&[], now), "No alerts are firing or pending.");
        let a = prom::Alert {
            name: "HighLatency".into(),
            state: "firing".into(),
            severity: "critical".into(),
            active_at: "2026-09-24T09:25:00Z".into(),
            summary: "p95 over 1s".into(),
            labels: [("service".to_string(), "checkout".to_string())].into_iter().collect(),
            value: "1.4".into(),
        };
        let text = alerts_text(&[a], now);
        assert!(text.starts_with("1 firing, 0 pending:\n- [critical] HighLatency — firing since"), "{text}");
        assert!(text.contains("(35 min)") && text.contains("p95 over 1s") && text.contains("service=\"checkout\""), "{text}");
        assert!(text.ends_with("nothing here silences or changes an alert."));
    }
