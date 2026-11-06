use super::*;

impl McpRegistry {
    pub fn new(data_dir: &Path) -> Self {
        McpRegistry { path: data_dir.join("mcp-servers.json") }
    }

    /// Record the tool surface a server presented, so the next attach can tell
    /// whether it changed. Best effort: a registry we cannot write is a reason to
    /// keep working, not to refuse the turn.
    pub fn remember_surface(&self, name: &str, fingerprint: &str) {
        let mut entries = self.list();
        if let Some(e) = entries.iter_mut().find(|e| e.name == name) {
            if e.tools_fingerprint == fingerprint {
                return;
            }
            e.tools_fingerprint = fingerprint.to_string();
            let _ = self.save(&entries);
        }
    }

    pub fn list(&self) -> Vec<McpEntry> {
        let Ok(text) = std::fs::read_to_string(&self.path) else { return Vec::new() };
        // If someone edits the file by hand and breaks it, the tab still loads: an
        // unreadable registry counts as empty, and the next save from the app repairs it.
        let mut v: Vec<McpEntry> = serde_json::from_str(&text).unwrap_or_default();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    pub(super) fn save(&self, entries: &[McpEntry]) -> bool {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Atomic for the reason the store audit gives: a truncating rewrite lets a
        // concurrent reader see an empty file and save that emptiness back — here
        // that would silently delete every server the user had configured.
        crate::core::store::write_json_atomic(&self.path, &entries).is_ok()
    }

    pub fn get(&self, name: &str) -> Option<McpEntry> {
        self.list().into_iter().find(|e| e.name == name)
    }

    /// Add or replace a server by name. Replacing rather than erroring is deliberate:
    /// re-pasting a corrected config is the normal way to fix a typo'd server.
    pub fn upsert(&self, name: &str, config: Value) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let mut entries = self.list();
        match entries.iter_mut().find(|e| e.name == name) {
            Some(e) => e.config = config,
            None => entries.push(McpEntry {
                name: name.to_string(),
                config,
                enabled: true,
                requires_approval: true,
                tools_fingerprint: String::new(),
            }),
        }
        self.save(&entries)
    }

    /// Change one server's flags. Each field is optional so the switches move
    /// independently — toggling a server off must not reset its approval setting.
    pub fn patch(
        &self,
        name: &str,
        enabled: Option<bool>,
        requires_approval: Option<bool>,
        config: Option<Value>,
    ) -> bool {
        let mut entries = self.list();
        let Some(e) = entries.iter_mut().find(|e| e.name == name) else { return false };
        if let Some(v) = enabled {
            e.enabled = v;
        }
        if let Some(v) = requires_approval {
            e.requires_approval = v;
        }
        if let Some(v) = config {
            e.config = v;
        }
        self.save(&entries)
    }

    pub fn remove(&self, name: &str) -> bool {
        let mut entries = self.list();
        let before = entries.len();
        entries.retain(|e| e.name != name);
        entries.len() != before && self.save(&entries)
    }
}
