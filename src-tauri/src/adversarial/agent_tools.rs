use crate::core::agent::TodoItem;
    use crate::core::agent_exec::dispatch_local;
    use std::sync::Mutex;

    fn call(root: &std::path::Path, name: &str, args: serde_json::Value) -> String {
        let todo: Mutex<Vec<TodoItem>> = Mutex::new(Vec::new());
        dispatch_local(root, &todo, name, &args).unwrap_or_default()
    }

    #[test]
    fn write_file_cannot_take_an_absolute_path() {
        // `write_file` rejects ".." on the raw string, then does root.join(path).
        // Path::join REPLACES the base when the argument is absolute, so an
        // absolute path escapes the workspace entirely.
        let root = super::scratch("agent-abs");
        let outside = super::scratch("agent-outside");
        let target = outside.join("owned.txt");

        let out = call(&root, "write_file", serde_json::json!({
            "path": target.to_string_lossy(), "content": "escaped"
        }));
        let escaped = target.exists();
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
        assert!(!escaped, "the agent wrote to an absolute path outside the workspace (tool said: {out:?})");
    }

    #[test]
    fn write_file_cannot_traverse_a_symlink() {
        let root = super::scratch("agent-link");
        let outside = super::scratch("agent-link-out");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("esc")).unwrap();

        let out = call(&root, "write_file", serde_json::json!({ "path": "esc/owned.txt", "content": "escaped" }));
        let escaped = outside.join("owned.txt").exists();
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
        assert!(!escaped, "the agent wrote through a symlink out of the workspace (tool said: {out:?})");
    }

    #[test]
    fn read_file_and_edit_file_stay_inside() {
        // The counterpart: these two route through coder::within, so they should
        // refuse. Confirms the gap is specific to write_file.
        let root = super::scratch("agent-read");
        let outside = super::scratch("agent-read-out");
        let secret = outside.join("secret.txt");
        std::fs::write(&secret, "TOPSECRET").unwrap();

        let r = call(&root, "read_file", serde_json::json!({ "path": secret.to_string_lossy() }));
        let e = call(&root, "edit_file", serde_json::json!({
            "path": secret.to_string_lossy(), "old_str": "TOPSECRET", "new_str": "x"
        }));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
        assert!(!r.contains("TOPSECRET"), "read_file leaked a file outside the workspace: {r:?}");
        assert!(e.starts_with("error"), "edit_file touched a file outside the workspace: {e:?}");
    }
