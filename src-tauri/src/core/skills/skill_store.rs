use super::*;

impl SkillStore {
    pub fn load(data_dir: &Path) -> Self {
        let mut skills = BTreeMap::new();
        // Built-ins first so a user skill of the same name replaces one.
        for (name, text) in BUILTIN_SKILLS {
            if let Ok(skill) = parse_skill(text, name, None) {
                skills.insert(skill.name.clone(), skill);
            }
        }
        if let Ok(rd) = fs::read_dir(data_dir.join("skills")) {
            for entry in rd.flatten() {
                let dir = entry.path();
                let md = dir.join("SKILL.md");
                if !md.is_file() {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&md) else { continue };
                let fallback = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(skill) = parse_skill(&text, fallback, Some(dir.to_string_lossy().into_owned())) {
                    skills.insert(skill.name.clone(), skill);
                }
                // A malformed skill is skipped rather than breaking the app.
            }
        }
        Self { skills, off: read_state(data_dir).off, data_dir: data_dir.to_path_buf() }
    }

    /// Every installed skill, switched on or not — for the Skills page. What an assistant may
    /// use is [`SkillStore::names_for`].
    pub fn names(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// Switched on: offered to assistants.
    pub fn is_on(&self, name: &str) -> bool {
        !self.off.contains(name)
    }

    /// Switch a skill on or off everywhere.
    pub fn set_on(&mut self, name: &str, on: bool) -> Result<(), String> {
        if !self.skills.contains_key(name) {
            return Err(format!("there's no skill called `{name}`"));
        }
        if on {
            self.off.remove(name);
        } else {
            self.off.insert(name.to_string());
        }
        write_state(&self.data_dir, &SkillState { off: self.off.clone() })
    }

    /// Forget a skill's switch, for a skill that is gone: a later one of the same name starts on,
    /// as any install does.
    pub fn forget(data_dir: &Path, name: &str) {
        let mut state = read_state(data_dir);
        if state.off.remove(name) {
            let _ = write_state(data_dir, &state);
        }
    }

    /// A skill an assistant may load now: installed, switched on, and on its list when it has
    /// one. What `load_skill` and a person's "/" pick both ask.
    pub fn usable(&self, name: &str, allowed: &[String]) -> Option<&Skill> {
        self.skills.get(name).filter(|s| self.is_on(&s.name) && (allowed.is_empty() || allowed.iter().any(|a| a == &s.name)))
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn catalog(&self) -> Vec<SkillCard> {
        self.skills.values().map(|s| SkillCard { name: s.name.clone(), description: s.description.clone() }).collect()
    }

    /// The catalog text injected into an agent's system context at session start.
    /// The catalogue an assistant is shown, narrowed to the skills it was given.
    ///
    /// Empty means everything installed — what every assistant saw before rosters
    /// existed, and still right for a generalist. A named list is for an assistant
    /// built around specific procedures: the Security Assistant offering to run
    /// `ui-component` is not a menu, it is noise, and a model reading a catalogue of
    /// twenty-one will reach for the wrong one eventually.
    ///
    /// A list naming a skill that is not installed narrows to nothing rather than
    /// falling back to everything — quietly widening a deliberate list is how an
    /// assistant ends up with procedures nobody gave it.
    pub fn catalog_text_for(&self, allowed: &[String]) -> String {
        if allowed.is_empty() {
            return self.catalog_text();
        }
        self.skills
            .values()
            .filter(|s| self.is_on(&s.name) && allowed.iter().any(|a| a == &s.name))
            .map(|s| format!("- {}: {}", s.name, s.description))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Skills a list names that are not installed.
    pub fn missing_from(&self, allowed: &[String]) -> Vec<String> {
        allowed.iter().filter(|a| !self.skills.contains_key(*a)).cloned().collect()
    }

    /// The names an assistant may use: its declared list, or everything installed — of the
    /// skills switched on, either way.
    pub fn names_for(&self, allowed: &[String]) -> Vec<String> {
        self.names().into_iter().filter(|n| self.is_on(n) && (allowed.is_empty() || allowed.iter().any(|a| a == n))).collect()
    }

    pub fn catalog_text(&self) -> String {
        let lines: Vec<String> = self.skills.values().filter(|s| self.is_on(&s.name)).map(|s| format!("- {}: {}", s.name, s.description)).collect();
        if lines.is_empty() {
            return String::new();
        }
        format!(
            "Available skills — call load_skill(name) to load one's full instructions when it's relevant to the task:\n{}",
            lines.join("\n")
        )
    }
}

/// The `load_skill` tool offered to every agent (progressive disclosure). Not a
/// persona-declared capability — it's added by the loop whenever skills exist.
pub fn load_skill_tool() -> ToolSpec {
    ToolSpec {
        name: "load_skill".into(),
        description: "Load a skill's full instructions by name. Call this when a skill from the catalog is relevant to the current task.".into(),
        schema: json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}),
        risk: RiskClass::Read,
        capability: "skills".into(),
    }
}
