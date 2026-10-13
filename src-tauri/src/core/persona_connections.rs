//! Which connected apps a assistant turns on by default.
//!
//! The middle of the three-level hierarchy: an account is connected globally, a
//! assistant decides whether ITS sessions start with that account enabled, and a
//! session can then override either way. This module is the middle one, and it was
//! the missing one — the GUI has rendered the toggles since the fork, the client
//! type declares `default_connections` as required, and the server never sent it.
//! Reading `.length` on that absent array is what took the persona screen down.
//!
//! Only connectors the assistant *recommends* appear. A assistant that never asked
//! for Slack has no opinion about Slack, and offering a default for it would invent
//! a decision nobody made. Unset means the assistant's own declaration stands: it
//! recommended the connector, so it wants it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One row of the assistant's connection defaults.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DefaultConnection {
    pub connector: String,
    pub enabled: bool,
    /// Whether the account is actually connected. A default for an account nobody
    /// has linked is a toggle that cannot mean anything yet, and the UI disables it
    /// rather than hiding it — "you could turn this on once you connect GitHub" is
    /// more useful than silence.
    pub connected: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    /// persona id -> connector id -> enabled
    #[serde(flatten)]
    by_persona: BTreeMap<String, BTreeMap<String, bool>>,
}

pub struct PersonaConnections {
    path: PathBuf,
}

impl PersonaConnections {
    pub fn load(data_dir: &Path) -> Self {
        Self { path: data_dir.join("persona-connections.json") }
    }

    fn read(&self) -> Stored {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn write(&self, s: &Stored) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
        crate::core::store::write_atomic(&self.path, &json).map_err(|e| e.to_string())
    }

    /// The assistant's defaults, one row per connector it recommends.
    ///
    /// `is_connected` is injected so this module never reaches into the credential
    /// store — it answers "what did the user choose", and nothing else.
    pub fn list(
        &self,
        persona: &str,
        recommended: &[String],
        is_connected: &dyn Fn(&str) -> bool,
    ) -> Vec<DefaultConnection> {
        let stored = self.read();
        let overrides = stored.by_persona.get(persona);
        recommended
            .iter()
            .map(|c| DefaultConnection {
                // Unset means on: the assistant declared it wants this connector, and
                // silently defaulting a recommendation to off would quietly ignore
                // the manifest.
                enabled: overrides.and_then(|o| o.get(c)).copied().unwrap_or(true),
                connected: is_connected(c),
                connector: c.clone(),
            })
            .collect()
    }

    pub fn set(&self, persona: &str, connector: &str, enabled: bool) -> Result<(), String> {
        if persona.trim().is_empty() || connector.trim().is_empty() {
            return Err("a persona and a connector are both required".into());
        }
        let mut stored = self.read();
        stored.by_persona.entry(persona.to_string()).or_default().insert(connector.to_string(), enabled);
        self.write(&stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("pconn-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn all_connected(_: &str) -> bool {
        true
    }

    #[test]
    fn a_recommended_connector_defaults_to_on() {
        // The assistant's manifest asked for it. Defaulting a recommendation to off
        // would quietly ignore the thing the manifest exists to say.
        let d = scratch();
        let s = PersonaConnections::load(&d);
        let rows = s.list("security", &["github".into()], &all_connected);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].enabled);
        assert!(rows[0].connected);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn turning_one_off_sticks_and_leaves_the_others_alone() {
        let d = scratch();
        let s = PersonaConnections::load(&d);
        s.set("security", "github", false).unwrap();
        let rows = s.list("security", &["github".into(), "slack".into()], &all_connected);
        assert!(!rows.iter().find(|r| r.connector == "github").unwrap().enabled);
        assert!(rows.iter().find(|r| r.connector == "slack").unwrap().enabled);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn assistants_do_not_share_each_others_choices() {
        let d = scratch();
        let s = PersonaConnections::load(&d);
        s.set("security", "github", false).unwrap();
        assert!(s.list("dep-audit", &["github".into()], &all_connected)[0].enabled);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_what_the_assistant_recommends_gets_a_row() {
        // An assistant that never asked for Slack has no opinion about Slack, and a
        // toggle for it would invent a decision nobody made.
        let d = scratch();
        let s = PersonaConnections::load(&d);
        s.set("security", "slack", true).unwrap();
        let rows = s.list("security", &["github".into()], &all_connected);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].connector, "github");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_unconnected_account_still_gets_a_row_that_says_so() {
        let d = scratch();
        let rows = PersonaConnections::load(&d).list("security", &["github".into()], &|_| false);
        assert!(!rows[0].connected, "the UI disables the toggle rather than hiding it");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_corrupt_file_reads_as_no_choices_rather_than_exploding() {
        let d = scratch();
        std::fs::write(d.join("persona-connections.json"), "{ not json").unwrap();
        assert!(PersonaConnections::load(&d).list("x", &["github".into()], &all_connected)[0].enabled);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_choice_survives_a_reload() {
        let d = scratch();
        PersonaConnections::load(&d).set("security", "github", false).unwrap();
        assert!(!PersonaConnections::load(&d).list("security", &["github".into()], &all_connected)[0].enabled);
        let _ = std::fs::remove_dir_all(&d);
    }
}
