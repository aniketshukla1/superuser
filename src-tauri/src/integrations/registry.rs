use super::*;

impl Registry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        for (id, name) in CATALOG {
            map.insert(
                id.to_string(),
                Integration {
                    id: id.to_string(),
                    name: name.to_string(),
                    status: "disconnected".into(),
                    last_sync: None,
                    memory_count: 0,
                    account: None,
                },
            );
        }
        Self { inner: Mutex::new(map) }
    }

    pub fn list(&self, memory: &MemoryEngine) -> Vec<Integration> {
        let counts = count_by_source(memory);
        let mut guard = self.inner.lock().unwrap();
        let mut out: Vec<Integration> = guard
            .values_mut()
            .map(|i| {
                i.memory_count = *counts.get(&i.id).unwrap_or(&0);
                let key = token_key(&i.id);
                // A source is "connected" once it has a stored credential.
                if secrets::has_integration_token(key) {
                    if i.status == "disconnected" {
                        i.status = "connected".into();
                    }
                    i.account = secrets::get_account(key);
                } else {
                    i.status = "disconnected".into();
                    i.account = None;
                }
                i.clone()
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Record that an integration finished a sync (after the real fetch).
    pub fn mark_synced(&self, id: &str, count: u32) {
        if let Some(integ) = self.inner.lock().unwrap().get_mut(id) {
            integ.memory_count = count;
            integ.last_sync = Some(chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string());
            integ.status = "connected".into();
        }
    }
}

/// Count memories whose category matches an integration's display name.
pub(super) fn count_by_source(memory: &MemoryEngine) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for m in memory.list() {
        let cat = m.category.to_lowercase();
        for (id, name) in CATALOG {
            if cat.contains(id) || cat.contains(&name.to_lowercase()) {
                *counts.entry(id.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}
