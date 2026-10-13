use super::*;

    #[test]
    #[ignore = "reads a real data directory; see the module note"]
    fn print_the_table() {
        let dir = match std::env::var("SUPERUSER_DATA_DIR") {
            Ok(d) if !d.trim().is_empty() => std::path::PathBuf::from(d),
            _ => match dirs::data_dir() {
                Some(d) => d.join("ai.superuser.app"),
                None => return eprintln!("no data dir"),
            },
        };
        let sessions = dir.join("sessions");
        let Ok(entries) = std::fs::read_dir(&sessions) else {
            return eprintln!("no sessions under {}", sessions.display());
        };

        let mut tally = Tally::new();
        let mut files = 0usize;
        for e in entries.flatten() {
            let log = e.path().join("run.jsonl");
            let Ok(text) = std::fs::read_to_string(&log) else { continue };
            files += 1;
            let sid = e.file_name().to_string_lossy().to_string();
            // Pair proposals with finishes inside one turn, in order. A name that was
            // proposed and never finished is a call that produced nothing.
            let mut proposed: Vec<String> = vec![];
            let mut finished: Vec<(String, bool)> = vec![];
            for line in text.lines() {
                let Ok(f) = serde_json::from_str::<serde_json::Value>(line) else { continue };
                let name = f["data"]["name"].as_str().unwrap_or("").to_string();
                if name.is_empty() {
                    continue;
                }
                match f["type"].as_str().unwrap_or("") {
                    "tool_proposed" => proposed.push(name),
                    "tool_finished" => {
                        let failed = is_failure(f["data"]["result_preview"].as_str().unwrap_or(""));
                        finished.push((name, failed));
                    }
                    _ => {}
                }
            }
            for (name, failed) in &finished {
                tally.record(name, *failed, &sid);
                if let Some(i) = proposed.iter().position(|p| p == name) {
                    proposed.remove(i);
                }
            }
            for name in proposed {
                tally.record_unfinished(&name, &sid);
            }
        }

        let catalogue = crate::core::agent::every_tool();
        let resident: Vec<&str> =
            catalogue.iter().map(|(_, n)| n.as_str()).filter(|n| crate::core::agent::is_resident(n)).collect();
        println!("\n{} session logs, {} tool calls\n", files, tally.total_calls());
        println!("{}", tally.render("Real usage", &catalogue, &resident));
    }
