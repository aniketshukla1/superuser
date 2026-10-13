use super::*;

    #[test]
    fn decodes_uddg_redirect() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage%3Fa%3D1&rut=x";
        assert_eq!(clean_url(href), "https://example.com/page?a=1");
    }

    #[test]
    fn parses_results_and_snippets() {
        let html = r#"
          <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F">The Rust Language</a>
          <a class="result__snippet" href="x">A language empowering everyone.</a>
          <a rel="nofollow" class="result__a" href="https://doc.rust-lang.org/book/">The Rust Book</a>
          <a class="result__snippet" href="x">Learn Rust here.</a>
        "#;
        let r = parse_results(html, 5);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].url, "https://rust-lang.org/");
        assert_eq!(r[0].title, "The Rust Language");
        assert_eq!(r[0].snippet, "A language empowering everyone.");
        assert_eq!(r[1].url, "https://doc.rust-lang.org/book/");
    }

    #[test]
    fn html_to_text_drops_script_and_tags() {
        let html = "<html><head><title>x</title></head><body><script>var a=1;</script><p>Hello <b>world</b></p></body></html>";
        let t = html_to_text(html);
        assert!(t.contains("Hello world"));
        assert!(!t.contains("var a"));
    }

    #[test]
    fn parses_searxng_json() {
        let json = serde_json::json!({
            "results": [
                { "title": "Rust", "url": "https://rust-lang.org/", "content": "A language." },
                { "title": "Book", "url": "https://doc.rust-lang.org/book/", "content": "Learn Rust." },
                { "title": "Bad", "url": "javascript:void(0)", "content": "skip me" }
            ]
        });
        let r = parse_searxng(&json, 5);
        assert_eq!(r.len(), 2); // the non-http result is dropped
        assert_eq!(r[0].url, "https://rust-lang.org/");
        assert_eq!(r[0].snippet, "A language.");
    }

    #[test]
    fn limit_is_respected() {
        let mut html = String::new();
        for i in 0..10 { html.push_str(&format!("<a class=\"result__a\" href=\"https://e{i}.com/\">T{i}</a>")); }
        assert_eq!(parse_results(&html, 3).len(), 3);
    }
