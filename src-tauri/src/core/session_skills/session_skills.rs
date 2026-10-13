use super::*;

/// One row of the composer's menu.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SessionSkillRow {
    pub name: String,
    pub description: String,
    /// Where the skill came from. Everything is `global` today — project-scoped
    /// skills are not a thing this app has yet — but the client's type has always
    /// carried the distinction, so it is answered honestly rather than omitted.
    pub scope: String,
    /// False = muted for this session only.
    pub enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Stored {
    /// session id -> skill name -> enabled. Only departures from "available".
    #[serde(flatten)]
    pub(super) by_session: BTreeMap<String, BTreeMap<String, bool>>,
}

pub struct SessionSkills {
    pub(super) path: PathBuf,
}

impl SessionSkills {
    pub fn load(data_dir: &Path) -> Self {
        Self { path: data_dir.join("session-skills.json") }
    }

    pub(super) fn read(&self) -> Stored {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub(super) fn write(&self, s: &Stored) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
        crate::core::store::write_atomic(&self.path, &json).map_err(|e| e.to_string())
    }

    /// The session's menu: the skills this assistant was given, with this session's
    /// mutes applied.
    ///
    /// `allowed` is the assistant's declared list; empty means everything installed.
    /// It narrows the menu for the same reason it narrows the prompt catalogue — and
    /// it must, or a person could force-run a skill the assistant was never given,
    /// which is the narrowing undone by the surface that displays it.
    pub fn menu(
        &self,
        session: &str,
        store: &crate::core::skills::SkillStore,
        allowed: &[String],
    ) -> Vec<SessionSkillRow> {
        let stored = self.read();
        let mutes = stored.by_session.get(session);
        store
            .names_for(allowed)
            .into_iter()
            .filter_map(|n| store.get(&n).cloned())
            .map(|s| SessionSkillRow {
                enabled: mutes.and_then(|m| m.get(&s.name)).copied().unwrap_or(true),
                scope: "global".into(),
                name: s.name,
                description: s.description,
            })
            .collect()
    }

    /// Is this skill usable in this session? The question a force-run has to ask
    /// before it runs anything.
    pub fn is_enabled(&self, session: &str, skill: &str) -> bool {
        self.read().by_session.get(session).and_then(|m| m.get(skill)).copied().unwrap_or(true)
    }

    pub fn set(&self, session: &str, skill: &str, enabled: bool) -> Result<(), String> {
        if session.trim().is_empty() || skill.trim().is_empty() {
            return Err("a session and a skill are both required".into());
        }
        let mut stored = self.read();
        stored.by_session.entry(session.to_string()).or_default().insert(skill.to_string(), enabled);
        self.write(&stored)
    }

    /// Forget a session's mute for one skill, returning it to "available".
    ///
    /// Distinct from setting it true: an explicit true and an absent entry mean the
    /// same thing today, but only the absent one keeps meaning it if the default
    /// ever changes.
    pub fn clear(&self, session: &str, skill: &str) -> Result<(), String> {
        let mut stored = self.read();
        if let Some(m) = stored.by_session.get_mut(session) {
            m.remove(skill);
            if m.is_empty() {
                stored.by_session.remove(session);
            }
        }
        self.write(&stored)
    }
}
