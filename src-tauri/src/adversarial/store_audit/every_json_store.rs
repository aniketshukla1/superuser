
    #[test]
    pub(super) fn every_json_store_writes_atomically() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders: Vec<String> = Vec::new();
        let mut files = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                    files.push(p);
                }
            }
        }
        // A test module can be a file of its own: `#[cfg(test)] mod driver;` puts
        // `driver.rs` and everything under `driver/` in test scope. Found first, so
        // the walk order cannot decide whether a file is audited.
        let mut test_paths: Vec<std::path::PathBuf> = Vec::new();
        for p in &files {
            let Ok(text) = std::fs::read_to_string(p) else { continue };
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let parent = p.parent().unwrap();
            let mod_dir = if matches!(stem, "mod" | "lib" | "main") { parent.to_path_buf() } else { parent.join(stem) };
            let mut pending = false;
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with("#[cfg(test)]") {
                    pending = true;
                } else if pending && !l.starts_with("#[") {
                    let decl = l.trim_start_matches("pub(crate) ").trim_start_matches("pub ");
                    if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
                        test_paths.push(mod_dir.join(format!("{name}.rs")));
                        test_paths.push(mod_dir.join(name));
                    }
                    pending = false;
                }
            }
        }
        for p in &files {
            {
                if test_paths.iter().any(|t| p.starts_with(t)) {
                    continue;
                }
                let name = p.file_name().and_then(|f| f.to_str()).unwrap_or("");
                // a module and, once split, the folder of the same name
                let in_module = |m: &str| name == format!("{m}.rs") || p.components().any(|c| c.as_os_str() == m);
                if ["store", "egress", "adversarial", "coder"].iter().any(|m| in_module(m)) {
                    continue; // the impl itself, an append-only log, this file, user-driven writes
                }
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                // Test modules build fixtures on disk; that's not a store. Two things
                // this has to get right, and the older rule got neither:
                //
                // It keys on `#[cfg(test)]`, not on the module's NAME. The old rule
                // knew only `mod tests` and `mod migration_tests`, so a module called
                // anything else — `mod calibration_tests`, say — had its fixtures
                // audited as though they were production writes.
                //
                // And it LEAVES test scope again. The old flag was set once and never
                // cleared, so every line after the first test module in a file went
                // unaudited — a real store written below one was invisible to this
                // check. Braces are counted so the scope actually closes.
                let mut in_tests = false;
                let mut pending_test = false;
                let mut depth: i32 = 0;
                for (i, line) in text.lines().enumerate() {
                    let l = line.trim_start();
                    if l.starts_with("#[cfg(test)]") {
                        pending_test = true;
                    } else if pending_test && !l.starts_with("#[") && !l.contains('{') {
                        // `#[cfg(test)] mod x;` or a one-line item: no scope opens here
                        pending_test = false;
                    }
                    if pending_test && l.contains('{') {
                        pending_test = false;
                        in_tests = true;
                        depth = 0;
                    }
                    if in_tests {
                        depth += line.matches('{').count() as i32;
                        depth -= line.matches('}').count() as i32;
                        if depth <= 0 {
                            in_tests = false;
                        }
                        continue;
                    }
                    if l.starts_with("//") {
                        continue;
                    }
                    // A *store* write, as opposed to writing a file the user asked
                    // for: the destination is a computed store path or a .json.
                    let is_write = line.contains("fs::write(") && !line.contains("resolve_write_path");
                    let looks_like_a_store = line.contains(".json")
                        || line.contains("_path(")
                        || line.contains("store_path")
                        || line.contains("index_path");
                    if is_write && looks_like_a_store {
                        offenders.push(format!("{}:{}  {}", name, i + 1, l.chars().take(90).collect::<String>()));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these rewrite a whole store with a truncating write, so a concurrent reader can see \
             an empty file and save the emptiness back:\n  {}",
            offenders.join("\n  ")
        );
    }
