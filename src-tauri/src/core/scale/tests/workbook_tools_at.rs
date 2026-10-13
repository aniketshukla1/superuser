use super::*;

    pub(super) fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-scale-{tag}-{nanos}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A vault of `n` memories, mostly synced (which is what a real one looks
    /// like: 117 of the 125 in the measured vault came from one connector).
    pub(super) fn build_vault(dir: &PathBuf, n: usize) -> MemoryEngine {
        let m = MemoryEngine::new(dir.clone());
        for i in 0..n {
            let internal = i % 20 == 0;
            m.save(&MemoryInput {
                filename: Some(if internal { format!("note-{i}.md") } else { format!("github-{i}.md") }),
                title: format!("Deploy pipeline note {i}"),
                category: if internal { "Notes".into() } else { "GitHub".into() },
                tags: vec!["deploy".into(), "build".into()],
                summary: format!("Something about the build, number {i}."),
                body: "The deploy pipeline runs on push to main. ".repeat(20),
                origin: if internal { Origin::Internal } else { Origin::External },
                key: None,
                supersedes: None,
            })
            .unwrap();
        }
        m
    }

    /// The sweep runs on turn 1 of **every** new conversation, so its cost is paid
    /// by the user in latency before the first token. This is the one on the hot
    /// path, and the one worth watching.
    /// What the spreadsheet tools do when a workbook stops being a toy.
    ///
    /// TOOLS-RESEARCH-2026 §5: our cell-level design is what SpreadsheetBench
    /// rewards, and SpreadsheetBench 2 moved the bar to instances averaging 11.8
    /// sheets and 593.5 modified cells. Nothing here had ever been measured at that
    /// size — the vault has a scale test and the workbook path did not.
    ///
    /// Prints rather than asserts tightly, like everything else in this file. The one
    /// assertion is a complexity-class guard: writing 500 cells must not cost 500
    /// times a single write.
    #[test]
    #[ignore]
    pub(super) fn workbook_tools_at_benchmark_scale() {
        use crate::core::sheets;

        let dir = scratch("workbook");
        let path = dir.join("big.xlsx");
        let names: Vec<String> = (0..12).map(|i| format!("Sheet{i}")).collect();

        let t = Instant::now();
        sheets::create(&path, &names).unwrap();
        println!("create 12 sheets:            {:>8.1?}", t.elapsed());

        // One write of 500 cells, which is the shape a real task has.
        let edits: Vec<sheets::CellEdit> = (0..500)
            .map(|i| sheets::CellEdit {
                cell: format!("A{}", i + 1),
                value: format!("{}", i * 7),
                result: None,
            })
            .collect();
        let t = Instant::now();
        let n = sheets::write_cells(&path, Some("Sheet0"), &edits).unwrap();
        let bulk = t.elapsed();
        println!("write 500 cells in one call: {:>8.1?}  ({n} cells)", bulk);

        // The same work one call at a time, which is the shape the *tool* has: the
        // model gets one `sheet_write` per step. 20 is enough to extrapolate without
        // spending a minute on it.
        let t = Instant::now();
        for i in 0..20 {
            let one = vec![sheets::CellEdit {
                cell: format!("B{}", i + 1),
                value: "x".into(),
                result: None,
            }];
            sheets::write_cells(&path, Some("Sheet1"), &one).unwrap();
        }
        let drip = t.elapsed();
        println!(
            "write 20 cells one call each: {:>8.1?}  (~{:.1?} per call — 500 that way would be ~{:.0?})",
            drip,
            drip / 20,
            drip / 20 * 500
        );

        let t = Instant::now();
        let rows = sheets::read_rows(&path, Some("Sheet0"), 1000, 50).unwrap();
        println!("read 500 rows back:          {:>8.1?}  ({} rows)", t.elapsed(), rows.len());

        let t = Instant::now();
        let s = sheets::sheet_names(&path).unwrap();
        println!("list 12 sheet names:         {:>8.1?}  ({} sheets)", t.elapsed(), s.len());

        assert!(
            bulk.as_millis() < 30_000,
            "500 cells in one call took {bulk:?} — this is one ordinary task's worth of work"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[ignore]
    pub(super) fn supercontext_sweep_at_vault_scale() {
        for n in [200usize, 1_000, 5_000] {
            let dir = scratch(&format!("sweep-{n}"));
            let m = build_vault(&dir, n);

            let t = Instant::now();
            let brief = crate::core::supercontext::sweep(&m, "what happened with the deploy pipeline");
            let took = t.elapsed();
            println!(
                "sweep over {n:>5} memories: {:>8.1?}  ({} inlined, {} sources withheld)",
                took,
                brief.inlined.len(),
                brief.withheld.len()
            );
            assert!(
                took.as_millis() < 10_000,
                "a sweep over {n} memories took {took:?} — this is on turn 1 of every conversation"
            );
            // The assertion this test was missing. It printed `0 inlined` at 1,000
            // and at 5,000 on every run for months, and only the timings were
            // checked — so a brief that had quietly stopped carrying any content
            // looked exactly like a brief that was working. The fixture vault has
            // internal memories at every size; at least one of them must survive.
            assert!(
                !brief.inlined.is_empty(),
                "a sweep over {n} memories inlined nothing — the tier that carries content is empty"
            );
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// `recall` is the same read the agent's own `recall_memory` does, and the
    /// sweep's cost is essentially two of them. Measured separately so a
    /// regression can be attributed to the right layer.
    #[test]
    #[ignore]
    pub(super) fn memory_recall_and_list_at_vault_scale() {
        for n in [200usize, 1_000, 5_000] {
            let dir = scratch(&format!("recall-{n}"));
            let m = build_vault(&dir, n);

            let t = Instant::now();
            let hits = m.recall("deploy pipeline", 24);
            let recall = t.elapsed();

            let t = Instant::now();
            let all = m.list();
            let list = t.elapsed();

            println!("{n:>5} memories: recall {recall:>8.1?}, list {list:>8.1?}  ({} hits, {} listed)", hits.len(), all.len());
            std::fs::remove_dir_all(&dir).ok();
        }
    }
