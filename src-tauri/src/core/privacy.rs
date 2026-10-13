//! Privacy Guard — deterministic, fully-local PII detection and redaction. No
//! model, no network: just regexes, so it's fast and auditable. Used both as a
//! standalone tool (paste → redact) and as a pre-flight scrub before text is
//! sent to a cloud model.

use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

struct Pat { kind: &'static str, label: &'static str, re: Regex }

// Order matters: match the most specific things first (SSN before generic
// number runs, cards before phones) so a value is only redacted once.
fn patterns() -> &'static [Pat] {
    static PATTERNS: OnceLock<Vec<Pat>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let p = |kind, label, src: &str| Pat { kind, label, re: Regex::new(src).unwrap() };
        vec![
            p("email", "[EMAIL]", r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"),
            p("ssn", "[SSN]", r"\b\d{3}-\d{2}-\d{4}\b"),
            p("card", "[CARD]", r"\b(?:\d[ \-]?){13,19}\b"),
            p("iban", "[IBAN]", r"\b[A-Z]{2}\d{2}[A-Z0-9]{10,30}\b"),
            // Dotted quads only when every octet is in range — otherwise a semver
            // like "1.2.3.4" or a build number reads as an address and the model
            // receives "[IP]" where the text said something specific.
            p("ip", "[IP]", r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b"),
            p("key", "[KEY]", r"\b(?:sk|pk|ghp|xox[baprs]|AKIA)[A-Za-z0-9_\-]{16,}\b"),
            // Phone numbers, narrowed. The previous pattern was any digit run of
            // nine-plus characters that could contain spaces or dashes, which ate
            // dates ("2024-01-01"), order ids, and columns of timings — and the
            // damage is not cosmetic, since this text is what reaches the model.
            //
            // So: an optional +country and 7-14 digits, in the groupings people
            // actually write, and never a bare run of digits split by single
            // spaces. A date is excluded outright rather than relied on to fail
            // the shape, because ISO dates are common in exactly this text.
            // (No lookaround — the `regex` crate has none — so every alternative
            // is anchored with `\b` and spells out its own separators.)
            p("phone", "[PHONE]", concat!(
                r"(?:\+\d{1,3}[\s.\-]?)?",                       // +1, +44 …
                r"(?:\(\d{3}\)[\s.\-]?\d{3}[\s.\-]?\d{4}",       // (415) 555-2671
                r"|\b\d{3}[\s.\-]\d{3}[\s.\-]\d{4}\b",           // 555-123-4567
                r"|\b\d{5}[\s.\-]\d{6}\b",                       // 07700 900123
                r"|\b\d{10,14}\b)",                              // 5551234567
            )),
        ]
    })
}

#[derive(Serialize)]
pub struct Finding { pub kind: String, pub label: String, pub count: usize }

#[derive(Serialize)]
pub struct RedactOut { pub redacted: String, pub findings: Vec<Finding>, pub total: usize }

// A card match must look like a real card: 13-19 digits and pass the Luhn check.
fn luhn_ok(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if !(13..=19).contains(&digits.len()) { return false; }
    let mut sum = 0u32;
    for (i, d) in digits.iter().rev().enumerate() {
        let mut v = *d;
        if i % 2 == 1 { v *= 2; if v > 9 { v -= 9; } }
        sum += v;
    }
    sum % 10 == 0
}

/// Credentials only — not the full PII sweep.
///
/// For text that is going to be *stored* rather than sent somewhere: a run
/// journal, a log line. The difference matters. [`redact`] also strips emails,
/// phone numbers and IPs, which is right before handing text to a cloud model and
/// wrong for a record of what a run did — a journal of a turn spent drafting email
/// would come back as a wall of `[EMAIL]` and be useless for the one job it has.
///
/// A leaked key, though, is permanent and remote. So this masks the shapes that
/// are always secrets and nothing else: provider key prefixes, bearer tokens, and
/// `password=` / `token=` / `api_key=` style assignments.
pub fn redact_secrets(text: &str) -> String {
    static SECRET: OnceLock<Vec<Regex>> = OnceLock::new();
    let pats = SECRET.get_or_init(|| {
        vec![
            // Provider-issued keys, same shapes the full sweep knows about.
            Regex::new(r"\b(?:sk|pk|ghp|gho|ghs|ghu|ghr|xox[baprs]|AKIA|ASIA)[A-Za-z0-9_\-]{16,}\b").unwrap(),
            // Slack app-level tokens (`xapp-…`), which don't match the `xox*` shape.
            Regex::new(r"\bxapp-[A-Za-z0-9\-]{10,}\b").unwrap(),
            // Google/Firebase API keys. Canonically `AIza` + 35, but the bound is
            // loose: a key one character off the spec is still a key.
            Regex::new(r"\bAIza[A-Za-z0-9_\-]{30,}\b").unwrap(),
            // JWTs. Three base64url segments; the header always starts `eyJ`.
            Regex::new(r"\beyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\b").unwrap(),
            // Vendor-prefixed keys that share one shape: Groq, HuggingFace,
            // Anthropic, OpenRouter, Replicate, Fireworks, Together.
            Regex::new(r"\b(?:gsk|hf|sk-ant|sk-or|r8|fw|together)[_\-][A-Za-z0-9_\-]{20,}\b").unwrap(),
            // `Authorization: Bearer …` and bare bearer tokens.
            Regex::new(r"(?i)\bbearer\s+[A-Za-z0-9._\-]{12,}").unwrap(),
            // key=value assignments where the key names a secret. `aws_secret_access_key`
            // is spelled out because its value is unprefixed base64 with no shape of
            // its own — the name is the only thing that marks it.
            Regex::new(r#"(?i)\b(?:password|passwd|secret|token|api[_-]?key|aws_secret_access_key|client[_-]?secret)\b["']?\s*[:=]?\s*["']?[^\s"',;)]{6,}"#).unwrap(),
            // PEM blocks. Whole-block, multiline, because a private key leaked in
            // pieces is still a private key.
            Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----").unwrap(),
        ]
    });
    let mut out = text.to_string();
    for re in pats {
        out = re.replace_all(&out, "[REDACTED]").into_owned();
    }
    out
}

/// Redact PII in `text`, returning the scrubbed string and per-kind counts.
pub fn redact(text: &str) -> RedactOut {
    let mut out = text.to_string();
    let mut findings: Vec<Finding> = Vec::new();
    let mut total = 0;
    for pat in patterns().iter() {
        let mut count = 0;
        // Rebuild the string, replacing matches (validating cards via Luhn).
        let mut result = String::with_capacity(out.len());
        let mut last = 0;
        for m in pat.re.find_iter(&out) {
            if pat.kind == "card" && !luhn_ok(m.as_str()) { continue; }
            result.push_str(&out[last..m.start()]);
            result.push_str(pat.label);
            last = m.end();
            count += 1;
        }
        result.push_str(&out[last..]);
        out = result;
        if count > 0 {
            total += count;
            findings.push(Finding { kind: pat.kind.into(), label: pat.label.into(), count });
        }
    }
    RedactOut { redacted: out, findings, total }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_go_but_ordinary_content_survives() {
        // A journal of a run that drafted email has to still read as one.
        let kept = "Drafted a reply to alice@example.com about the 555-123-4567 line.";
        assert_eq!(redact_secrets(kept), kept);

        assert!(redact_secrets("export KEY=sk-abcdefghijklmnopqrstuvwx").contains("[REDACTED]"));
        assert!(!redact_secrets("token: ghp_abcdefghijklmnopqrstuvwxyz01").contains("ghp_"));
        assert!(!redact_secrets("Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.abc").contains("eyJhbG"));
        assert!(!redact_secrets("password=hunter2000").contains("hunter2000"));
        assert!(!redact_secrets("api_key = 'abcd1234efgh'").contains("abcd1234efgh"));
    }

    #[test]
    fn redacts_common_pii() {
        let t = "Email me at jane.doe@acme.co or call +1 (415) 555-2671. SSN 123-45-6789.";
        let r = redact(t);
        assert!(r.redacted.contains("[EMAIL]"));
        assert!(r.redacted.contains("[PHONE]"));
        assert!(r.redacted.contains("[SSN]"));
        assert!(!r.redacted.contains("jane.doe@acme.co"));
        assert!(!r.redacted.contains("123-45-6789"));
        assert!(r.total >= 3);
    }

    #[test]
    fn only_luhn_valid_cards_are_cards() {
        // 4242 4242 4242 4242 is a valid Luhn test card
        let r = redact("card 4242 4242 4242 4242 end");
        assert!(r.redacted.contains("[CARD]"), "valid card should redact: {}", r.redacted);
        // a random 16-digit run that fails Luhn should NOT be flagged as a card
        let r2 = redact("ref 1111 1111 1111 1112");
        assert!(!r2.redacted.contains("[CARD]"));
    }

    #[test]
    fn clean_text_is_untouched() {
        let r = redact("Just a normal sentence with no secrets.");
        assert_eq!(r.total, 0);
        assert_eq!(r.redacted, "Just a normal sentence with no secrets.");
    }
}
