use super::*;

    pub(super) fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    pub(super) fn durations_read_the_way_prometheus_and_people_write_them() {
        assert_eq!(parse_duration("30s"), Ok(30));
        assert_eq!(parse_duration("1h30m"), Ok(5_400));
        assert_eq!(parse_duration("7d"), Ok(604_800));
        assert_eq!(parse_duration("2 hours"), Ok(7_200));
        assert_eq!(parse_duration(" 1 week "), Ok(604_800));
        for bad in ["", "90", "h", "3x", "-1h", "0m"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
        assert_eq!((prom_duration(5_400), prom_duration(7_200), prom_duration(604_800), prom_duration(45)), ("90m".into(), "2h".into(), "1w".into(), "45s".into()));
    }

    #[test]
    pub(super) fn moments_read_as_written_and_never_in_the_future() {
        let now = at("2026-09-24T10:00:00Z");
        assert_eq!(parse_when("now", now), Ok(now));
        assert_eq!(parse_when("3h", now), Ok(at("2026-09-24T07:00:00Z")));
        assert_eq!(parse_when("90m ago", now), Ok(at("2026-09-24T08:30:00Z")));
        assert_eq!(parse_when("2026-09-23T14:00:00+02:00", now), Ok(at("2026-09-23T12:00:00Z")));
        assert_eq!(parse_when("1790150400", now), Ok(Utc.timestamp_opt(1_790_150_400, 0).unwrap()));
        let local_2pm = Local.with_ymd_and_hms(2026, 9, 23, 14, 0, 0).unwrap().with_timezone(&Utc);
        assert_eq!(parse_when("2026-09-23 14:00", now), Ok(local_2pm), "this machine's time zone");
        let yesterday = now.with_timezone(&Local).date_naive().pred_opt().unwrap().and_hms_opt(14, 0, 0).unwrap();
        assert_eq!(parse_when("yesterday 14:00", now), Ok(Local.from_local_datetime(&yesterday).unwrap().with_timezone(&Utc)));
        assert!(parse_when("2026-09-25T10:00:00Z", now).unwrap_err().contains("future"));
        assert!(parse_when("after lunch", now).is_err());
    }

    #[test]
    pub(super) fn a_range_asks_for_about_240_points_on_a_round_step() {
        assert_eq!(step_for(3_600), 15);
        assert_eq!(step_for(2 * 3_600), 30);
        assert_eq!(step_for(86_400), 600);
        assert_eq!(step_for(14 * 86_400), 7_200);
    }

    #[test]
    pub(super) fn a_step_is_clear_a_spike_is_possible_and_wobble_is_steady() {
        let before = [0.20, 0.21, 0.19, 0.20, 0.22, 0.20];
        let step = [0.40, 0.42, 0.39, 0.41, 0.40, 0.43];
        let s = shift(&before, &step).unwrap();
        assert_eq!(s.verdict, Verdict::Clear);
        assert_eq!(s.beyond, 1.0);
        assert!((s.pct.unwrap() - 100.0).abs() < 5.0);

        let spike = [0.20, 0.21, 0.95, 0.20, 0.19, 0.21];
        assert_eq!(shift(&before, &spike).unwrap().verdict, Verdict::Possible, "one bad point isn't a step");

        let wobble = [0.21, 0.19, 0.20, 0.22, 0.20, 0.21];
        assert_eq!(shift(&before, &wobble).unwrap().verdict, Verdict::Steady);

        let down = [0.10, 0.11, 0.09, 0.10, 0.10, 0.11];
        let d = shift(&before, &down).unwrap();
        assert_eq!((d.verdict, d.change < 0.0), (Verdict::Clear, true), "a drop is a shift too");

        assert_eq!(shift(&[1.0, 1.0], &[2.0, 2.0, 2.0]).unwrap().verdict, Verdict::TooLittle);
        assert_eq!(shift(&[1.0, 1.0, 1.0], &[1.02, 1.02, 1.02]).unwrap().verdict, Verdict::Steady, "a flat line nudged 2% hasn't moved");
        assert_eq!(shift(&[0.0, 0.0, 0.0], &[3.0, 3.0, 3.0]).unwrap().verdict, Verdict::Clear, "from nothing to something");
        assert!(shift(&[f64::NAN, f64::NAN], &[1.0]).is_none(), "NaN isn't a value");
    }

    #[test]
    pub(super) fn numbers_read_at_three_significant_figures() {
        assert_eq!(fmt_num(0.213456), "0.213");
        assert_eq!(fmt_num(0.000123456), "0.000123");
        assert_eq!(fmt_num(12.3456), "12.3");
        assert_eq!(fmt_num(1234.4), "1234");
        assert_eq!(fmt_num(2.0), "2");
        assert_eq!(fmt_num(-0.5), "-0.5");
        assert_eq!(fmt_num(f64::NAN), "NaN");
        assert_eq!(fmt_num(3.2e9), "3.200e9");
    }

    #[test]
    pub(super) fn a_charts_csv_lines_series_up_by_time_and_keeps_formulas_out() {
        let s = |pts: &[(f64, f64)]| Series { labels: BTreeMap::new(), points: pts.to_vec() };
        let a = s(&[(1_790_150_400.0, 0.2), (1_790_150_460.0, f64::NAN)]);
        let b = s(&[(1_790_150_460.0, -1.5)]);
        let csv = to_csv(&[("/checkout".into(), &a), ("=cmd|x".into(), &b)]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "time,/checkout,'=cmd|x");
        assert!(lines[1].ends_with(",0.2,"), "no point is an empty cell: {}", lines[1]);
        assert!(lines[2].ends_with(",,-1.5"), "NaN is left out, a negative stays a number: {}", lines[2]);
        assert_eq!(csv_num(0.1 + 0.2), "0.3");
    }

    /// The chart tool reads the kept CSV itself (`dataload`), so it has to come out as a time
    /// column and number columns — not text that happens to look like numbers.
    #[test]
    pub(super) fn the_kept_csv_is_what_a_chart_reads_as_numbers() {
        let dir = std::env::temp_dir().join(format!("su-perf-csv-{}", crate::core::inbox::short_id()));
        let a = Series { labels: BTreeMap::new(), points: vec![(1_790_150_400.0, 0.25), (1_790_150_460.0, -0.5)] };
        let rel = keep_csv(&dir, "compare", "histogram_quantile(0.95, sum by (le) (rate(http_seconds_bucket[5m])))", at("2026-09-24T09:00:00Z"), &to_csv(&[("/checkout".into(), &a)])).unwrap();
        assert_eq!(rel, "metrics/compare-histogram_quantile-http_seconds_bucket-090000.csv");
        let rows = crate::core::dataload::read(&dir, &rel).unwrap();
        assert_eq!(rows.columns, vec!["time", "/checkout"]);
        assert_eq!(rows.rows[0]["/checkout"], serde_json::json!(0.25));
        assert_eq!(rows.rows[1]["/checkout"], serde_json::json!(-0.5));
        assert!(rows.rows[0]["time"].is_string());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn signals_are_a_name_and_a_query_a_line() {
        let (list, problems) = signals(
            "# the ones that page\nCheckout p95: histogram_quantile(0.95, sum by (le) (rate(http_seconds_bucket[5m])))\n\nErrors: sum(rate(http_requests_total{code=~\"5..\"}[5m]))\nno colon here\n",
        );
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Checkout p95");
        assert!(list[1].query.starts_with("sum(rate("));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("line 5"), "{problems:?}");
        let many: String = (0..25).map(|i| format!("S{i}: up\n")).collect();
        let (list, problems) = signals(&many);
        assert_eq!(list.len(), MAX_SIGNALS);
        assert!(problems[0].contains("first 20"));
    }

    pub(super) fn series(handler: &str, pts: Vec<(f64, f64)>) -> Series {
        Series { labels: [("handler".to_string(), handler.to_string())].into_iter().collect(), points: pts }
    }

    #[test]
    pub(super) fn a_comparison_ranks_what_moved_and_keeps_its_data_for_a_chart() {
        let dir = std::env::temp_dir().join(format!("su-perf-compare-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Conn::from_fields(&serde_json::json!({ "url": "http://prom.internal:9090" })).unwrap();
        let change = at("2026-09-24T09:00:00Z");
        let t = |min: i64| (change.timestamp() + min * 60) as f64;
        let before = |base: f64| (-10..0).map(|m: i64| (t(m), base + m.rem_euclid(2) as f64 * 0.01)).collect::<Vec<_>>();
        let after = |base: f64| (0..10).map(|m: i64| (t(m), base + m.rem_euclid(2) as f64 * 0.01)).collect::<Vec<_>>();
        let answer = Answer::Series(
            vec![
                series("/cart", [before(0.20), after(0.20)].concat()),
                series("/checkout", [before(0.20), after(0.60)].concat()),
                series("/v2/pay", after(0.30)),
                series("/legacy", before(0.10)),
            ],
            4,
        );
        let out = compare_text(&conn, "q", change - chrono::Duration::minutes(10), change, change + chrono::Duration::minutes(10), 60, answer, &[], &dir, change).unwrap();
        let text = &out.text;
        assert!(text.contains("Clear shifts (1):\n1. /checkout — up 195%: 0.205 → 0.605"), "{text}");
        assert!(text.contains("Within its usual variation (1): /cart — up 0%"), "{text}");
        assert!(text.contains("New after the change (1): /v2/pay"), "{text}");
        assert!(text.contains("Gone after the change (1): /legacy"), "{text}");
        assert!(text.contains("Open it in Prometheus: http://prom.internal:9090/graph?"), "{text}");
        assert_eq!(out.files.len(), 1);
        let csv = std::fs::read_to_string(dir.join(&out.files[0])).unwrap();
        assert!(csv.starts_with("time,/checkout,/cart,/v2/pay,/legacy\n"), "the one that moved most comes first: {csv}");
        assert!(text.contains(&format!("source: \\\"{}\\\"", out.files[0])) || text.contains(&format!("source: \"{}\"", out.files[0])), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
