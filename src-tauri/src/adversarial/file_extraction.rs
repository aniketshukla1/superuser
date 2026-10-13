use crate::core::files::extract;

    #[test]
    fn a_malformed_office_file_errors_instead_of_panicking() {
        let d = super::scratch("files-bad");
        let cases: [(&str, &[u8]); 4] = [
            ("bad.docx", b"PK\x03\x04 not really a zip"),
            ("bad.xlsx", &[0u8; 64]),
            ("bad.pdf", b"%PDF-1.4 garbage \xff\xfe"),
            ("empty.docx", b""),
        ];
        let mut panicked = Vec::new();
        for (name, bytes) in cases {
            let p = d.join(name);
            std::fs::write(&p, bytes).unwrap();
            let path = p.to_string_lossy().into_owned();
            if std::panic::catch_unwind(move || extract(&path)).is_err() {
                panicked.push(name);
            }
        }
        std::fs::remove_dir_all(&d).ok();
        assert!(panicked.is_empty(), "extract() panicked on malformed input: {panicked:?}");
    }

    #[test]
    fn a_zip_bomb_does_not_exhaust_memory() {
        // A .docx is a zip; extract_ooxml_text reads entries whole. A highly
        // compressible entry expands ~1000x, so a 1 MB file becomes 1 GB.
        let d = super::scratch("files-bomb");
        let p = d.join("bomb.docx");
        {
            use std::io::Write;
            let f = std::fs::File::create(&p).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let opts: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            z.start_file("word/document.xml", opts).unwrap();
            // 200 MB of zeros -> a few hundred KB on disk.
            let chunk = vec![b'0'; 1 << 20];
            for _ in 0..200 {
                z.write_all(&chunk).unwrap();
            }
            z.finish().unwrap();
        }
        let on_disk = std::fs::metadata(&p).unwrap().len();
        let path = p.to_string_lossy().into_owned();
        let t = std::time::Instant::now();
        let out = extract(&path);
        let ms = t.elapsed().as_millis();
        let produced = out.as_ref().map(|e| e.text.as_ref().map(|t| t.len()).unwrap_or(0)).unwrap_or(0);
        std::fs::remove_dir_all(&d).ok();
        assert!(
            produced < 16 * 1024 * 1024,
            "a {on_disk}-byte file expanded to {produced} bytes in memory ({ms} ms) — no decompression cap"
        );
    }
