use super::*;

impl Learned {
    pub fn load(data_dir: &Path) -> Self {
        let inner = std::fs::read_to_string(facets_path(data_dir))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { data_dir: data_dir.to_path_buf(), inner, form: load(data_dir) }
    }

    pub(in crate::core::profile) fn save(&self) {
        // The learner writes this from a background pass while a new conversation
        // may be reading it for its frozen block.
        let _ = crate::core::store::write_json_atomic(&facets_path(&self.data_dir), &self.inner);
    }

    pub fn form(&self) -> &Profile {
        &self.form
    }

    /// Record evidence. Silently ignores anything the user has forgotten, so a
    /// dismissed belief doesn't come straight back on the next pass — that's the
    /// difference between forgetting something and deleting a row.
    pub fn observe(&mut self, obs: Vec<Observation>) -> usize {
        let mut added = 0;
        for mut o in obs {
            o.value = clean_value(&o.value);
            o.key = clean_key(&o.key);
            if o.value.is_empty() || o.key.is_empty() || self.is_forgotten(o.class, &o.key, &o.value) {
                continue;
            }
            o.evidence = clean_evidence(&o.evidence);
            self.inner.observations.push(o);
            added += 1;
        }
        if added > 0 {
            if self.inner.observations.len() > OBSERVATION_LIMIT {
                let excess = self.inner.observations.len() - OBSERVATION_LIMIT;
                self.inner.observations.drain(0..excess);
            }
            self.save();
        }
        added
    }

    pub(in crate::core::profile) fn is_forgotten(&self, class: Class, key: &str, value: &str) -> bool {
        self.inner
            .forgotten
            .iter()
            .any(|v| v.class == class && v.key == key && v.value.eq_ignore_ascii_case(value))
    }

    pub(in crate::core::profile) fn is_pinned(&self, class: Class, key: &str, value: &str) -> bool {
        self.inner
            .pinned
            .iter()
            .any(|v| v.class == class && v.key == key && v.value.eq_ignore_ascii_case(value))
    }

    /// Shield a facet from decay, or stop shielding it. A pinned facet is always
    /// Active and always renders first.
    pub fn set_pinned(&mut self, class: Class, key: &str, value: &str, pinned: bool) {
        let (key, value) = (clean_key(key), clean_value(value));
        self.inner.pinned.retain(|v| !(v.class == class && v.key == key && v.value.eq_ignore_ascii_case(&value)));
        if pinned {
            self.inner.pinned.push(Verdict { class, key, value, at: now() });
        }
        self.save();
    }

    /// Drop a belief and block it from returning.
    ///
    /// The supporting observations go too. Keeping them would mean the evidence
    /// still showed up as "why it thinks that" for something it is no longer
    /// allowed to think, and the whole point of the surface is that those two
    /// things match.
    pub fn forget(&mut self, class: Class, key: &str, value: &str) {
        let (key, value) = (clean_key(key), clean_value(value));
        self.inner
            .observations
            .retain(|o| !(o.class == class && o.key == key && o.value.eq_ignore_ascii_case(&value)));
        self.inner.pinned.retain(|v| !(v.class == class && v.key == key && v.value.eq_ignore_ascii_case(&value)));
        if !self.is_forgotten(class, &key, &value) {
            self.inner.forgotten.push(Verdict { class, key, value, at: now() });
        }
        self.save();
    }

    /// Let a forgotten belief be learned again.
    pub fn unforget(&mut self, class: Class, key: &str, value: &str) {
        let (key, value) = (clean_key(key), clean_value(value));
        self.inner
            .forgotten
            .retain(|v| !(v.class == class && v.key == key && v.value.eq_ignore_ascii_case(&value)));
        self.save();
    }

    pub fn forgotten(&self) -> &[Verdict] {
        &self.inner.forgotten
    }

    /// Every belief, scored and ranked, Active first.
    ///
    /// Competition is per class+key: the strongest value is Active and the rest are
    /// Provisional, still tracked, free to overtake it later. That is what "a
    /// dropped belief can re-earn its place" means in practice — nothing is ever
    /// deleted for losing, so being said once more is enough to flip it back.
    pub fn facets(&self) -> Vec<Facet> {
        self.facets_at(now())
    }
}
