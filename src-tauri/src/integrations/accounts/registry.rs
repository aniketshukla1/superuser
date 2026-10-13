use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    /// Stable id within the connector (e.g. an email or workspace id).
    pub id: String,
    /// Human label shown in the UI.
    pub label: String,
    pub default: bool,
}

/// The pure registry — no I/O, so it's fully testable.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub accounts: Vec<Account>,
}

impl Registry {
    pub fn parse(stored: Option<&str>) -> Self {
        stored
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    /// Add or update an account. The first one added becomes the default.
    pub fn upsert(&mut self, id: &str, label: &str) {
        if let Some(a) = self.accounts.iter_mut().find(|a| a.id == id) {
            a.label = label.to_string();
            return;
        }
        let first = self.accounts.is_empty();
        self.accounts.push(Account { id: id.to_string(), label: label.to_string(), default: first });
    }

    /// Remove an account; if it was the default, the first remaining one takes over.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(pos) = self.accounts.iter().position(|a| a.id == id) else {
            return false;
        };
        let was_default = self.accounts[pos].default;
        self.accounts.remove(pos);
        if was_default {
            if let Some(first) = self.accounts.first_mut() {
                first.default = true;
            }
        }
        true
    }

    pub fn set_default(&mut self, id: &str) -> bool {
        if !self.accounts.iter().any(|a| a.id == id) {
            return false;
        }
        for a in &mut self.accounts {
            a.default = a.id == id;
        }
        true
    }

    pub fn default_id(&self) -> Option<String> {
        self.accounts
            .iter()
            .find(|a| a.default)
            .or_else(|| self.accounts.first())
            .map(|a| a.id.clone())
    }
}
