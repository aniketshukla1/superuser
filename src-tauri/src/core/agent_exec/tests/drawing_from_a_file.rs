use super::*;

        #[test]
        fn a_chart_can_be_drawn_from_the_file_it_names() {
            let root = scratch();
            std::fs::write(root.join("q3.csv"), "quarter,revenue\nQ1,1200\nQ2,1400\nQ3,1850\n").unwrap();
            let out = dispatch_local(
                &root,
                &Mutex::new(Vec::new()),
                "chart",
                &json!({"spec": {"type": "bar", "title": "Revenue", "x": {"field": "quarter"}, "y": {"field": "revenue"}}, "source": "q3.csv"}),
            )
            .expect("chart is a local tool");
            assert!(out.contains("3 row(s) read out of the file") && out.contains("quarter, revenue"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn a_table_with_no_rows_and_no_file_is_told_both_ways_of_giving_it_some() {
            let root = scratch();
            let out = dispatch_local(
                &root,
                &Mutex::new(Vec::new()),
                "table",
                &json!({"spec": {"columns": [{"field": "a", "label": "A"}]}, "source": "the figures in the user's message"}),
            )
            .unwrap();
            assert!(out.starts_with("error:") && out.contains("or name a .csv"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn a_file_that_cannot_be_read_says_so_rather_than_drawing_nothing() {
            let root = scratch();
            let out = dispatch_local(
                &root,
                &Mutex::new(Vec::new()),
                "table",
                &json!({"spec": {"columns": [{"field": "a", "label": "A"}]}, "source": "missing.csv"}),
            )
            .unwrap();
            assert!(out.contains("couldn't read `missing.csv`"), "got: {out}");
            let _ = std::fs::remove_dir_all(&root);
        }
