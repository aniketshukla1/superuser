use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bba-files-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn reads_text_files() {
        let p = tmp_file("notes.md", b"# Hello\nsome notes");
        let e = extract(p.to_str().unwrap()).unwrap();
        assert_eq!(e.kind, "text");
        assert!(e.text.unwrap().contains("some notes"));
    }

    #[test]
    fn encodes_images_as_base64() {
        // bytes don't need to be a valid PNG for the reader to base64 them
        let p = tmp_file("shot.png", &[0x89, 0x50, 0x4e, 0x47, 1, 2, 3]);
        let e = extract(p.to_str().unwrap()).unwrap();
        assert_eq!(e.kind, "image");
        assert_eq!(e.mime, "image/png");
        assert!(e.image_base64.is_some());
        assert!(e.text.is_none());
    }

    #[test]
    fn flags_unsupported_binary() {
        let p = tmp_file("blob.bin", &[0, 159, 146, 150, 0, 1]);
        let e = extract(p.to_str().unwrap()).unwrap();
        assert_eq!(e.kind, "unsupported");
        assert!(e.note.is_some());
    }

    #[test]
    fn missing_file_errors() {
        assert!(extract("/no/such/file.txt").is_err());
    }

    #[test]
    fn strips_ooxml_to_text() {
        let xml = "<w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p>";
        let t = strip_xml_text(xml);
        assert!(t.contains("Hello"));
        assert!(t.contains("World"));
    }

    #[test]
    fn reads_docx_text() {
        // build a minimal .docx (a zip with word/document.xml)
        let p = tmp_file("doc.docx", &[]);
        let f = std::fs::File::create(&p).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        zw.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        zw.write_all(br#"<?xml version="1.0"?><w:document><w:body><w:p><w:r><w:t>Quarterly report draft</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        zw.finish().unwrap();
        let e = extract(p.to_str().unwrap()).unwrap();
        assert_eq!(e.kind, "text");
        assert!(e.text.unwrap().contains("Quarterly report draft"));
    }
