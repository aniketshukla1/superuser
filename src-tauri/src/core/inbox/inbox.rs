use super::*;

impl Inbox {
    pub fn load(data_dir: &Path) -> Self {
        let asks = fs::read_to_string(data_dir.join("inbox.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { data_dir: data_dir.to_path_buf(), asks }
    }

    pub(super) fn save(&self) {
        let _ = crate::core::store::write_json_atomic(&self.data_dir.join("inbox.json"), &self.asks);
    }

    pub fn park(&mut self, ask: Ask) -> Ask {
        self.asks.push(ask.clone());
        self.save();
        ask
    }

    pub fn pending(&self) -> Vec<Ask> {
        self.asks.iter().filter(|a| a.status == "pending").cloned().collect()
    }

    pub fn list(&self) -> Vec<Ask> {
        self.asks.clone()
    }

    pub fn get(&self, id: &str) -> Option<&Ask> {
        self.asks.iter().find(|a| a.id == id)
    }

    /// Approve or deny a parked ask. Returns the updated ask.
    pub fn resolve(&mut self, id: &str, approve: bool) -> Result<Ask, String> {
        let ask = self.asks.iter_mut().find(|a| a.id == id).ok_or_else(|| format!("unknown ask: {id}"))?;
        if ask.status != "pending" {
            return Err(format!("that request was already {}", ask.status));
        }
        ask.status = if approve { "approved" } else { "denied" }.to_string();
        ask.resolved_at = Some(now());
        let out = ask.clone();
        self.save();
        Ok(out)
    }

    /// Drop resolved asks older than `max_age_secs` (housekeeping).
    pub fn prune(&mut self, max_age_secs: f64) {
        let cutoff = now() - max_age_secs;
        let before = self.asks.len();
        self.asks.retain(|a| a.status == "pending" || a.resolved_at.map(|t| t > cutoff).unwrap_or(true));
        if self.asks.len() != before {
            self.save();
        }
    }
}
