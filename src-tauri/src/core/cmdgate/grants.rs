use std::collections::{HashMap, HashSet};
    use std::sync::{Mutex, OnceLock};

    #[derive(Default)]
    struct Grants {
        /// Tools blessed wholesale — "Always allow" on the card.
        tools: HashSet<String>,
        /// Command prefixes blessed — "Always allow this command".
        commands: Vec<String>,
    }

    fn table() -> &'static Mutex<HashMap<String, Grants>> {
        static T: OnceLock<Mutex<HashMap<String, Grants>>> = OnceLock::new();
        T.get_or_init(Default::default)
    }

    fn lock() -> std::sync::MutexGuard<'static, HashMap<String, Grants>> {
        table().lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn allow_tool(session: &str, tool: &str) {
        lock().entry(session.to_string()).or_default().tools.insert(tool.to_string());
    }

    pub fn tool_allowed(session: &str, tool: &str) -> bool {
        lock().get(session).is_some_and(|g| g.tools.contains(tool))
    }

    /// Bless one command as a prefix rule. Matching is token-wise
    /// ([`super::auto_allowable`]), so approving `cargo test` covers
    /// `cargo test --release` but never `cargo testfoo` — and approving
    /// `rm -rf build` never covers `rm -rf build2`.
    pub fn allow_command(session: &str, command: &str) {
        let c = command.trim().to_string();
        if c.is_empty() {
            return;
        }
        let mut t = lock();
        let g = t.entry(session.to_string()).or_default();
        if !g.commands.contains(&c) {
            g.commands.push(c);
        }
    }

    pub fn commands(session: &str) -> Vec<String> {
        lock().get(session).map(|g| g.commands.clone()).unwrap_or_default()
    }

    /// Forget everything this session was granted. Called when a session is
    /// deleted; also the escape hatch for tests, which share the process.
    pub fn clear(session: &str) {
        lock().remove(session);
    }
