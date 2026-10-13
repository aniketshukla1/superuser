use super::*;

    #[test]
    fn the_address_is_the_servers_root_whatever_page_was_pasted() {
        let c = |url: &str| Conn::from_fields(&json!({ "url": url })).map(|c| c.url);
        assert_eq!(c("http://prom.internal:9090/").unwrap(), "http://prom.internal:9090");
        assert_eq!(c("http://prom.internal:9090/graph").unwrap(), "http://prom.internal:9090");
        assert_eq!(c("http://prom.internal:9090/api/v1/query?query=up").unwrap(), "http://prom.internal:9090");
        assert_eq!(c("https://x.grafana.net/api/prom").unwrap(), "https://x.grafana.net/api/prom", "a path prefix is kept");
        assert_eq!(c("http://prom.internal:9090/graph?g0.expr=up&g0.tab=0").unwrap(), "http://prom.internal:9090", "a page with its query");
        assert!(c("prom.internal:9090").unwrap_err().contains("http://"), "no guessing at the scheme");
        assert!(c("").is_err());
        assert!(c("ftp://prom").is_err());
    }

    #[test]
    fn a_login_is_one_of_the_ways_in_and_complete() {
        let with = |v: Value| Conn::from_fields(&v);
        assert!(with(json!({ "url": "http://p:9090", "username": "123456", "password": "glc_x" })).is_ok(), "Grafana Cloud's way");
        assert!(with(json!({ "url": "http://p:9090", "token": "t" })).is_ok());
        assert!(with(json!({ "url": "http://p:9090", "org_id": "team-a" })).is_ok(), "Mimir's tenant alone");
        assert!(with(json!({ "url": "http://p:9090", "username": "u" })).unwrap_err().contains("password"));
        assert!(with(json!({ "url": "http://p:9090", "password": "p" })).unwrap_err().contains("username"));
        assert!(with(json!({ "url": "http://p:9090", "username": "u", "password": "p", "token": "t" })).unwrap_err().contains("not both"));
        let inline = with(json!({ "url": "https://ada:p%40ss@prom.example.com" })).unwrap();
        assert_eq!((inline.url.as_str(), inline.username.as_str(), inline.password.as_str()), ("https://prom.example.com", "ada", "p@ss"),
            "a login in the address becomes the login, and leaves the address");
        let c = with(json!({ "url": "http://p:9090", "token": "t", "org_id": "a" })).unwrap();
        assert_eq!(Conn::from_fields(&serde_json::from_str(&c.to_stored()).unwrap()).unwrap(), c, "stored and read back the same");
    }

    #[test]
    fn answers_of_every_kind_are_read() {
        let vector = json!({"resultType": "vector", "result": [
            {"metric": {"__name__": "up", "job": "api"}, "value": [1727000000.5, "1"]},
            {"metric": {"job": "db"}, "value": [1727000000.5, "NaN"]},
        ]});
        let Answer::Series(s, total) = parse_result(&vector).unwrap() else { panic!() };
        assert_eq!((total, s[0].points.clone()), (2, vec![(1727000000.5, 1.0)]));
        assert!(s[1].points[0].1.is_nan());

        let matrix = json!({"resultType": "matrix", "result": [
            {"metric": {"handler": "/checkout"}, "values": [[1, "0.2"], [2, "+Inf"], [3, "0.4"]]},
        ]});
        let Answer::Series(s, _) = parse_result(&matrix).unwrap() else { panic!() };
        assert_eq!(s[0].points.len(), 3);
        assert_eq!(s[0].points[1].1, f64::INFINITY);

        assert_eq!(parse_result(&json!({"resultType": "scalar", "result": [5, "2.5"]})).unwrap(), Answer::Scalar(5.0, 2.5));
        assert!(parse_result(&json!({"resultType": "streams", "result": []})).is_err());
    }

    #[test]
    fn the_api_says_what_went_wrong_in_a_sentence() {
        let bad = r#"{"status":"error","errorType":"bad_data","error":"1:5: parse error: unexpected \")\""}"#;
        assert!(envelope(400, bad, "p:9090").unwrap_err().starts_with("the query didn't parse"));
        let slow = r#"{"status":"error","errorType":"timeout","error":"query timed out in expression evaluation"}"#;
        assert!(envelope(503, slow, "p:9090").unwrap_err().contains("Narrow it"));
        assert!(envelope(401, "Unauthorized", "p:9090").unwrap_err().contains("refused the login"));
        assert!(envelope(404, "<html>404</html>", "p:9090").unwrap_err().contains("no Prometheus API"));
        assert!(envelope(200, r#"{"status":"success","data":{}}"#, "p:9090").is_ok());
    }

    #[test]
    fn alerts_come_firing_first_with_what_they_say() {
        let data = json!({"alerts": [
            {"labels": {"alertname": "DiskFilling", "severity": "warning", "instance": "db-1"}, "annotations": {"summary": "Disk 85% full"},
             "state": "pending", "activeAt": "2026-09-24T08:00:00Z", "value": "8.5e-01"},
            {"labels": {"alertname": "HighLatency", "severity": "critical", "service": "checkout"}, "annotations": {"description": "p95 over 1s"},
             "state": "firing", "activeAt": "2026-09-24T09:00:00Z", "value": "1.4e+00"},
        ]});
        let a = parse_alerts(&data);
        assert_eq!((a[0].name.as_str(), a[0].severity.as_str(), a[0].summary.as_str()), ("HighLatency", "critical", "p95 over 1s"));
        assert_eq!(a[0].labels.get("service").map(String::as_str), Some("checkout"));
        assert!(!a[0].labels.contains_key("alertname"));
        assert_eq!(a[1].state, "pending");
    }

    #[test]
    fn series_are_named_by_what_tells_them_apart() {
        let l = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>();
        let a = l(&[("__name__", "http_seconds"), ("handler", "/checkout"), ("job", "api")]);
        let b = l(&[("__name__", "http_seconds"), ("handler", "/cart"), ("job", "api")]);
        assert_eq!(short_names(&[&a, &b]), vec!["/checkout", "/cart"]);
        assert_eq!(short_names(&[&a]), vec![r#"http_seconds{handler="/checkout", job="api"}"#], "one series: its whole name");
        assert_eq!(short_names(&[&l(&[])]), vec!["value"]);
        assert_eq!(fmt_labels(&l(&[("__name__", "up")])), "up");
        assert_eq!(fmt_labels(&l(&[("job", "api")])), r#"{job="api"}"#);
    }

    #[test]
    fn a_link_opens_the_same_query_on_the_servers_own_graph_page() {
        let c = Conn::from_fields(&json!({ "url": "http://prom.internal:9090" })).unwrap();
        let link = c.graph_link("rate(http_requests_total[5m])", 1727170200.0, 7200).unwrap();
        assert!(link.starts_with("http://prom.internal:9090/graph?g0.expr=rate%28http_requests_total%5B5m%5D%29"), "{link}");
        assert!(link.contains("g0.range_input=2h") && link.contains("g0.end_input=2024-09-24+09%3A30%3A00"), "{link}");
        let cloud = Conn::from_fields(&json!({ "url": "https://x.grafana.net/api/prom", "username": "1", "password": "p" })).unwrap();
        assert_eq!(cloud.graph_link("up", 0.0, 60), None, "an API prefix has no graph page");
    }

    /// The whole path through a socket: a login sent the way it was asked for, and the
    /// envelope read. Loopback, so nothing leaves the machine.
    #[tokio::test]
    async fn a_query_goes_out_with_its_login_and_comes_back_read() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let seen2 = seen.clone();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap_or(0);
                *seen2.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = r#"{"status":"success","data":{"resultType":"vector","result":[{"metric":{"job":"api"},"value":[1727000000,"3"]}]},"warnings":["partial response"]}"#;
                let _ = s
                    .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes())
                    .await;
            }
        });
        let c = Conn::from_fields(&json!({ "url": format!("http://127.0.0.1:{port}"), "username": "ada", "password": "s3cret", "org_id": "team-a" })).unwrap();
        let (answer, warnings) = c.instant("sum by (job) (up)", 1727000000.0).await.unwrap();
        let Answer::Series(s, _) = answer else { panic!() };
        assert_eq!(s[0].points[0].1, 3.0);
        assert_eq!(warnings, vec!["partial response"]);
        let req = seen.lock().unwrap().clone();
        assert!(req.starts_with("GET /api/v1/query?query=sum+by+%28job%29+%28up%29&time=1727000000.000 "), "{req}");
        assert!(req.to_lowercase().contains("authorization: basic ywrhonmzy3jlda=="), "basic auth, as asked: {req}");
        assert!(req.to_lowercase().contains("x-scope-orgid: team-a"), "{req}");
    }
