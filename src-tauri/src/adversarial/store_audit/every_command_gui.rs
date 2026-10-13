
/// Every store that follows load -> mutate -> save must write atomically.
    /// `store.rs` exists precisely because `fs::write` truncates first, and a
    /// reader that catches the gap parses "" and writes the emptiness back.
    /// Every command the GUI invokes must exist.
    ///
    /// `invoke` swallows its error and returns null, so a command that was never
    /// registered does not fail — it returns nothing, forever, and the surface built
    /// on it looks like it is working. That is how a complete voice-input screen sat
    /// in front of eleven commands that did not exist, and how the MCP tab's buttons
    /// were all 404s behind a rendered list.
    ///
    /// This is the compile-time version of noticing: the frontend's literal invoke
    /// names are checked against `generate_handler!`.
    #[test]
    pub(super) fn every_command_the_gui_invokes_is_registered() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let gui = root.join("..").join("surfaces").join("gui").join("src");

        // Known gaps, recorded rather than hidden. Each is a surface whose backend
        // was never written; `invoke` returning null means they fail silently, so
        // listing them here is the only place they are visible. Removing an entry
        // when its command is implemented is the point — the list should shrink.
        // Empty, and the assertion below keeps it that way: an entry left here
        // after its command is implemented would imply a gap that is closed.
        const KNOWN_MISSING: &[&str] = &[];

        let mut invoked: std::collections::BTreeSet<String> = Default::default();
        let mut stack = vec![gui];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                if ext != "ts" && ext != "tsx" {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                // Tests stub the bridge with their own command names; those are
                // fixtures, not calls the app makes.
                let is_test = p.file_name().and_then(|f| f.to_str()).map(|n| n.contains(".test.")).unwrap_or(false);
                if is_test {
                    continue;
                }
                for (at, _) in text.match_indices("invoke") {
                    // Only literal calls: `invoke<T>("name")` / `invokeStrict("name")`.
                    // A name built at runtime cannot be checked here, and guessing at
                    // one would make this test lie.
                    // Clamp to a char boundary: these files are full of em dashes,
                    // and a fixed byte window lands inside one sooner or later.
                    let mut stop = (at + 120).min(text.len());
                    while stop < text.len() && !text.is_char_boundary(stop) {
                        stop += 1;
                    }
                    let rest = &text[at..stop];
                    let Some(open) = rest.find("(\"") else { continue };
                    // Anything but a type parameter between the word and the paren
                    // means this was some other identifier ending in "invoke".
                    let between = &rest[6..open];
                    let plausible = between.chars().all(|c| {
                        c.is_whitespace()
                            || c.is_alphanumeric()
                            || matches!(c, '<' | '>' | ',' | '_' | '|' | '.' | ':' | '[' | ']')
                    });
                    if !plausible {
                        continue;
                    }
                    let after = &rest[open + 2..];
                    let Some(end) = after.find('"') else { continue };
                    let name = &after[..end];
                    if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                        invoked.insert(name.to_string());
                    }
                }
            }
        }
        assert!(invoked.len() > 20, "the scanner found almost nothing ({}) — it has stopped working", invoked.len());

        // The handler list lives in the app module (lib.rs only declares modules).
        let lib = std::fs::read_to_string(root.join("src").join("lib.rs")).expect("lib.rs")
            + crate::test_support::module_source("app");
        let start = lib.find("generate_handler![").expect("generate_handler!");
        let block = &lib[start..];
        let end = block.find(']').unwrap_or(block.len());
        let block = &block[..end];
        let registered: std::collections::BTreeSet<String> = block
            .split(',')
            .filter_map(|t| t.trim().rsplit("::").next())
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        let missing: Vec<String> = invoked
            .iter()
            .filter(|n| !registered.contains(*n) && !KNOWN_MISSING.contains(&n.as_str()))
            .cloned()
            .collect();
        assert!(
            missing.is_empty(),
            "the GUI invokes these, and generate_handler! does not register them — `invoke` \
             returns null on a missing command, so each of these fails silently and the \
             surface built on it looks like it works: {missing:?}"
        );

        // And the debt list must stay honest: an entry that has since been
        // implemented should be deleted, not left implying a gap that is closed.
        let stale: Vec<&&str> = KNOWN_MISSING.iter().filter(|n| registered.contains(**n)).collect();
        assert!(stale.is_empty(), "these are registered now — remove them from KNOWN_MISSING: {stale:?}");
    }
