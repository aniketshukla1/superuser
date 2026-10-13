use crate::core::privacy::{redact, redact_secrets};

    #[test]
    fn modern_credential_shapes_are_masked() {
        let cases = [
            ("google api key", "AIzaSyA1234567890abcdefghijklmnopqrstu"),
            ("jwt", "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdefghijklmnopqrstuvwxyz012345"),
            ("slack app token", "xapp-1-A012345678-1234567890123-abcdefabcdefabcdefabcdef"),
            ("aws secret", "aws_secret_access_key wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            ("groq", "gsk_abcdefghijklmnopqrstuvwxyz0123456789ABCD"),
            ("hf token", "hf_abcdefghijklmnopqrstuvwxyz0123456789"),
            ("pem private key", "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA1234\n-----END RSA PRIVATE KEY-----"),
        ];
        let mut missed = Vec::new();
        for (what, secret) in cases {
            let out = redact_secrets(&format!("here it is: {secret} ok"));
            if out.contains(secret.split('\n').next().unwrap()) && out.contains(secret.trim_start_matches('-').split(['\n', ' ']).next().unwrap_or(secret)) {
                missed.push(what);
            }
        }
        assert!(missed.is_empty(), "redact_secrets let these through verbatim: {missed:?}");
    }

    #[test]
    fn ordinary_text_is_not_mangled_into_uselessness() {
        // Over-redaction is a correctness bug too: the cloud model gets nonsense.
        // Note: a bare "1.2.3.4" is a valid dotted quad and a plausible semver.
        // Regex can't tell them apart, so that one stays redacted by design; the
        // cases here are the ones where the shape really isn't an identifier.
        let cases = [
            ("an out-of-range quad", "build 999.888.777.666 of the tool"),
            ("a date range", "the sprint runs 2024-01-01 to 2024-03-31 inclusive"),
            ("an order id", "order 1234 5678 9012 3456 7 shipped"),
            ("a plain number run", "the build took 1200 300 900 ms across runs"),
            ("a table of timings", "p50 120 p95 450 p99 1200 ms"),
        ];
        let mut mangled = Vec::new();
        for (what, text) in cases {
            let out = redact(text);
            if out.total > 0 {
                mangled.push(format!("{what}: {:?} -> {:?}", text, out.redacted));
            }
        }
        assert!(mangled.is_empty(), "ordinary text was redacted:\n  {}", mangled.join("\n  "));
    }

    #[test]
    fn redaction_is_not_quadratic_on_a_large_document() {
        let big = "contact me at a@b.com or +1 555 123 4567. ".repeat(20_000);
        let t = std::time::Instant::now();
        let out = redact(&big);
        let ms = t.elapsed().as_millis();
        assert!(out.total > 0);
        assert!(ms < 3_000, "redacting a {} KB document took {ms} ms", big.len() / 1024);
    }
