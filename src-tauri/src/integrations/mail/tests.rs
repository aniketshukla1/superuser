use super::*;

    #[test]
    fn a_plain_message_reads_as_written_and_its_headers_unfold() {
        let raw = b"From: Ada <ada@example.com>\r\nSubject: Quarterly\r\n review\r\nMessage-ID: <1@x>\r\n\r\nHello,\r\nsee you Monday.\r\n";
        let p = parse(raw);
        assert_eq!(p.decoded("subject"), "Quarterly review");
        assert_eq!(p.header("Message-ID"), Some("<1@x>"));
        assert_eq!(p.text.replace("\r\n", "\n").trim(), "Hello,\nsee you Monday.");
        assert_eq!(address(p.header("from").unwrap()), "ada@example.com");
    }

    #[test]
    fn the_plain_part_wins_over_html_and_quoted_printable_is_decoded() {
        let raw = concat!(
            "Content-Type: multipart/alternative; boundary=\"b1\"\r\n\r\n",
            "--b1\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\n",
            "It=E2=80=99s done. This line is soft=\r\n broken.\r\n",
            "--b1\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>HTML version</p>\r\n",
            "--b1--\r\n"
        );
        let p = parse(raw.as_bytes());
        assert_eq!(p.text.trim(), "It\u{2019}s done. This line is soft broken.");
    }

    #[test]
    fn an_html_only_message_in_base64_reads_as_its_words() {
        let html = "<html><style>p{color:red}</style><body><p>Hi&nbsp;there</p><p>Invoice &amp; receipt</p><script>x()</script></body></html>";
        let raw = format!(
            "Content-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
            B64.encode(html)
        );
        let p = parse(raw.as_bytes());
        assert_eq!(p.text, "Hi there\n\nInvoice & receipt");
    }

    #[test]
    fn nested_parts_find_the_text_and_skip_the_attachment() {
        let raw = concat!(
            "Content-Type: multipart/mixed; boundary=outer\n\n",
            "--outer\nContent-Type: multipart/alternative; boundary=inner\n\n",
            "--inner\nContent-Type: text/plain\n\nThe body.\n",
            "--inner\nContent-Type: text/html\n\n<b>The body.</b>\n",
            "--inner--\n",
            "--outer\nContent-Type: text/plain\nContent-Disposition: attachment; filename=notes.txt\n\nNOT THIS\n",
            "--outer--\n"
        );
        let p = parse(raw.as_bytes());
        assert_eq!(p.text.trim(), "The body.");
    }

    #[test]
    fn encoded_words_and_latin1_decode() {
        assert_eq!(decode_words("=?UTF-8?B?SGVsbG8g8J+Riw==?="), "Hello \u{1F44B}");
        assert_eq!(decode_words("=?iso-8859-1?Q?Caf=E9_ouvert?= today"), "Caf\u{e9} ouvert today");
        assert_eq!(decode_words("=?utf-8?q?two?= =?utf-8?q?_words?="), "two words", "the space between words belongs to neither");
        assert_eq!(decode_words("plain =? not a word"), "plain =? not a word");
        let raw = b"Content-Type: text/plain; charset=iso-8859-1\n\nna\xefve\n";
        assert_eq!(parse(raw).text.trim(), "na\u{ef}ve");
    }

    #[test]
    fn addresses_come_out_of_every_shape_lowercased() {
        assert_eq!(address("Ada Lovelace <ADA@Example.com>"), "ada@example.com");
        assert_eq!(address("\"Lovelace, Ada\" <ada@example.com>"), "ada@example.com");
        assert_eq!(address("ada@example.com"), "ada@example.com");
        assert_eq!(address("no address here"), "");
    }

    #[test]
    fn quoted_history_is_cut_and_the_text_is_capped() {
        let text = "Can you send the deck?\n\nThanks\n\nOn Mon, 4 Aug 2026 at 09:00, Bo <bo@x.com> wrote:\n> old stuff\n> more";
        assert_eq!(strip_quoted(text), "Can you send the deck?\n\nThanks");
        assert_eq!(strip_quoted("Yes.\n> quoted inline\nDone."), "Yes.\nDone.");
        assert_eq!(strip_quoted("Top\n-----Original Message-----\nFrom: x"), "Top");
        let long = "a".repeat(TEXT_CAP + 50);
        assert!(strip_quoted(&long).ends_with("[…the rest of this email was cut]"));
    }
