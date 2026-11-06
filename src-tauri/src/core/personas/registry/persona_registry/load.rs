use super::*;

impl PersonaRegistry {
    pub fn load(data_dir: &Path) -> Self {
        let mut order = Vec::new();
        let mut entries = HashMap::new();
        for m in builtins() {
            order.push(m.id.clone());
            entries.insert(m.id.clone(), m);
        }
        // Before reading them: assistants saved under the product's old word still
        // say "coworker" in text the app itself generated. See `migrate_wording`.
        migrate_wording(data_dir);
        // User personas: data_dir/personas/*.md (may override a built-in by id).
        if let Ok(rd) = fs::read_dir(data_dir.join("personas")) {
            let mut mds: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
                .collect();
            mds.sort();
            for md in mds {
                let Ok(text) = fs::read_to_string(&md) else { continue };
                let stem = md.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(m) =
                    parse_manifest(&text, Some(stem), false, Some(md.to_string_lossy().into_owned()))
                {
                    if !entries.contains_key(&m.id) {
                        order.push(m.id.clone());
                    }
                    entries.insert(m.id.clone(), m);
                }
                // A malformed user persona is skipped rather than breaking the app.
            }
        }
        let state = load_state(data_dir);
        // The user's choices win over the shipped manifest. Applied here, once, so
        // every reader — the prompt builder, the tool expander, the detail page —
        // sees one answer rather than each remembering to check for an override.
        for (id, m) in entries.iter_mut() {
            if let Some(mode) = state.mode.get(id) {
                if VALID_MODES.contains(&mode.as_str()) {
                    m.default_permission_mode = mode.clone();
                }
            }
            if let Some(roster) = state.subagents.get(id) {
                m.subagents = roster.clone();
            }
            if let Some(&on) = state.browser.get(id) {
                apply_browser_grant(m, on);
            }
        }
        Self { data_dir: data_dir.to_path_buf(), order, entries, state }
    }

    pub(in crate::core::personas) fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.state) {
            let _ = crate::core::store::write_atomic(&self.data_dir.join("personas.json"), &json);
        }
    }

    pub fn manifest(&self, id: &str) -> Option<&PersonaManifest> {
        self.entries.get(id)
    }

    /// Built-ins are enabled by default; user personas are opt-in. Explicit choice wins.
    pub fn is_enabled(&self, id: &str) -> bool {
        if let Some(&e) = self.state.enabled.get(id) {
            return e;
        }
        self.entries.get(id).map(|m| m.builtin).unwrap_or(false)
    }

    pub fn is_surfaced(&self, id: &str) -> bool {
        self.state.surfaced.get(id).copied().unwrap_or(true)
    }

    pub fn default_id(&self) -> String {
        if let Some(d) = self.state.default.as_deref() {
            if self.entries.contains_key(d) && self.is_enabled(d) {
                return d.to_string();
            }
        }
        if self.entries.contains_key(DEFAULT_PERSONA_ID) && self.is_enabled(DEFAULT_PERSONA_ID) {
            return DEFAULT_PERSONA_ID.to_string();
        }
        self.order
            .iter()
            .find(|id| self.is_enabled(id))
            .cloned()
            .unwrap_or_else(|| DEFAULT_PERSONA_ID.to_string())
    }

    /// The new-session picker: enabled AND surfaced, in registration order.
    pub fn sidebar(&self) -> Vec<PersonaPickerItem> {
        let default = self.default_id();
        self.order
            .iter()
            .filter_map(|id| {
                let m = self.entries.get(id)?;
                (self.is_enabled(id) && self.is_surfaced(id)).then(|| PersonaPickerItem {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    icon: m.icon.clone(),
                    tagline: m.tagline.clone(),
                    family: m.family.clone(),
                    needs_workspace: m.needs_workspace(),
                    default: m.id == default,
                })
            })
            .collect()
    }

    /// All installed personas with their lifecycle state, as the Personas settings panel lists them.
    pub fn list_all(&self) -> Vec<PersonaListItem> {
        let default = self.default_id();
        self.order
            .iter()
            .filter_map(|id| {
                let m = self.entries.get(id)?;
                Some(PersonaListItem {
                    kind: m.kind.clone(),
                    intro: m.intro.clone(),
                    starters: m.starters.clone(),
                    id: m.id.clone(),
                    name: m.name.clone(),
                    icon: m.icon.clone(),
                    tagline: m.tagline.clone(),
                    description: m.description.clone(),
                    family: m.family.clone(),
                    workspace: m.workspace.clone(),
                    needs_workspace: m.needs_workspace(),
                    tools: m.tools.clone(),
                    subagents: m.subagents.clone(),
                    skills: m.skills.clone(),
                    messaging: m.messaging,
                    connectors: m.connectors,
                    recommends: m.recommends.clone(),
                    builtin: m.builtin,
                    enabled: self.is_enabled(id),
                    surfaced: self.is_surfaced(id),
                    default: m.id == default,
                })
            })
            .collect()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown persona: {id}"));
        }
        self.state.enabled.insert(id.to_string(), enabled);
        if enabled {
            // Enabling implies surfacing — "enabled but invisible" is never what a user just asked for.
            self.state.surfaced.insert(id.to_string(), true);
        }
        self.save();
        Ok(())
    }

    pub fn set_surfaced(&mut self, id: &str, surfaced: bool) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown persona: {id}"));
        }
        self.state.surfaced.insert(id.to_string(), surfaced);
        self.save();
        Ok(())
    }

    /// Change how far this assistant may act without asking.
    ///
    /// The most consequential per-assistant setting there is, and it was displayed
    /// as static text on a page titled "Configure".
    pub fn set_mode(&mut self, id: &str, mode: &str) -> Result<(), String> {
        let mode = mode.trim().to_lowercase();
        if !VALID_MODES.contains(&mode.as_str()) {
            return Err(format!("{mode:?} is not a permission mode"));
        }
        if !self.entries.contains_key(id) {
            return Err(format!("unknown assistant: {id}"));
        }
        self.state.mode.insert(id.to_string(), mode.clone());
        if let Some(m) = self.entries.get_mut(id) {
            m.default_permission_mode = mode;
        }
        self.save();
        Ok(())
    }
}
