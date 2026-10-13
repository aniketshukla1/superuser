use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-projects-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn create_list_delete_roundtrip() {
        let d = tmp();
        let p = create(&d, "Launch", None).unwrap();
        assert!(p.id.starts_with("proj-"));
        assert_eq!(list(&d).len(), 1);
        delete(&d, &p.id);
        assert_eq!(list(&d).len(), 0);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn rejects_empty_name() {
        let d = tmp();
        assert!(create(&d, "   ", None).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn creates_a_backing_folder_when_missing() {
        let d = tmp();
        let folder = d.join("my-project-files");
        let p = create(&d, "Files", Some(folder.to_string_lossy().to_string())).unwrap();
        assert!(folder.exists(), "the backing folder should be created");
        assert_eq!(p.folder, Some(folder.to_string_lossy().to_string()));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn opening_the_same_folder_twice_is_one_project() {
        // `workspace_open` calls `create` on every open. This appended each time, so a
        // folder opened four times appeared four times in the gate's Recent list — and
        // React warned about the duplicate keys, because they really were the same row.
        let d = tmp();
        let f = d.join("repo").to_string_lossy().to_string();
        let first = create(&d, "repo", Some(f.clone())).unwrap();
        let again = create(&d, "repo", Some(f.clone())).unwrap();
        assert_eq!(first.id, again.id, "the second open must not mint a new project");
        assert_eq!(list(&d).len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn reopening_moves_a_folder_back_to_the_top() {
        // The whole point of the list. `created_at` cannot answer this — it is the same
        // number it was the day the folder was first opened.
        let d = tmp();
        let old = d.join("old").to_string_lossy().to_string();
        let new = d.join("new").to_string_lossy().to_string();
        create(&d, "old", Some(old.clone())).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        create(&d, "new", Some(new.clone())).unwrap();
        assert_eq!(list(&d)[0].folder.as_deref(), Some(new.as_str()));

        std::thread::sleep(std::time::Duration::from_millis(2));
        create(&d, "old", Some(old.clone())).unwrap(); // reopened
        assert_eq!(list(&d)[0].folder.as_deref(), Some(old.as_str()), "reopening is what makes a folder recent");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_list_is_newest_first_because_its_first_entry_is_the_boot_default() {
        // `/v1/health` builds `default_workspace` from `list().first()`. Sorted the
        // other way — as it was — a cold boot adopted the OLDEST folder ever opened.
        let d = tmp();
        let a = d.join("a").to_string_lossy().to_string();
        let b = d.join("b").to_string_lossy().to_string();
        create(&d, "a", Some(a)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        create(&d, "b", Some(b.clone())).unwrap();
        assert_eq!(list(&d).first().and_then(|p| p.folder.clone()), Some(b));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_store_written_before_last_opened_existed_still_orders() {
        // Every install already has rows without the field. They must not all collapse
        // to the same instant and sort arbitrarily.
        let d = tmp();
        let raw = r#"[
          {"id":"proj-1","name":"older","folder":"/tmp/older","created_at":1000},
          {"id":"proj-2","name":"newer","folder":"/tmp/newer","created_at":2000}
        ]"#;
        std::fs::write(d.join("projects.json"), raw).unwrap();
        let got = list(&d);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "newer", "falls back to created_at, newest first");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn duplicates_already_on_disk_are_collapsed_on_read() {
        // The fix has to work without a migration: these rows are already written on
        // every install that has been used.
        let d = tmp();
        let raw = r#"[
          {"id":"proj-1","name":"repo","folder":"/tmp/repo","created_at":1000},
          {"id":"proj-2","name":"repo","folder":"/tmp/repo","created_at":2000},
          {"id":"proj-3","name":"repo","folder":"/tmp/repo","created_at":3000}
        ]"#;
        std::fs::write(d.join("projects.json"), raw).unwrap();
        let got = list(&d);
        assert_eq!(got.len(), 1, "one folder is one row");
        assert_eq!(got[0].id, "proj-3", "and it is the most recent of them");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn two_folderless_projects_are_two_projects() {
        // Deduping is on the FOLDER. Projects with no folder share a key of "nothing"
        // and must not collapse into each other.
        let d = tmp();
        create(&d, "Launch", None).unwrap();
        create(&d, "Hiring", None).unwrap();
        assert_eq!(list(&d).len(), 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn rename_updates_in_place() {
        let d = tmp();
        let p = create(&d, "Old", None).unwrap();
        rename(&d, &p.id, "New");
        assert_eq!(list(&d)[0].name, "New");
        std::fs::remove_dir_all(&d).ok();
    }
