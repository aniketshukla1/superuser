use super::*;

impl Learned {
    pub(in crate::core::profile) fn facets_at(&self, at: f64) -> Vec<Facet> {
        let mut groups: HashMap<(Class, String, String), Vec<&Observation>> = HashMap::new();
        let form = self.form.observations();
        for o in form.iter().chain(self.inner.observations.iter()) {
            if self.is_forgotten(o.class, &o.key, &o.value) {
                continue;
            }
            // Case-insensitive grouping on the value, so "Concise" and "concise"
            // are one belief rather than two competing halves of one.
            groups.entry((o.class, o.key.clone(), o.value.to_lowercase())).or_default().push(o);
        }

        let mut facets: Vec<Facet> = groups
            .into_values()
            .map(|obs| {
                let newest = obs.iter().max_by(|a, b| a.at.total_cmp(&b.at)).unwrap();
                let strongest = obs.iter().map(|o| o.cue).min_by_key(|c| match c {
                    Cue::Explicit => 0,
                    Cue::Structural => 1,
                    Cue::Behavioral => 2,
                }).unwrap();
                let class = newest.class;
                let pinned = self.is_pinned(class, &newest.key, &newest.value);
                let score = if pinned {
                    f64::INFINITY
                } else {
                    obs.iter()
                        .map(|o| {
                            // A form field does not age. It is a standing
                            // declaration, not something that happened once, and
                            // decaying it would mean the profile you filled in
                            // quietly stopped applying while still sitting there on
                            // screen. (It can still be *outweighed* by enough
                            // evidence to the contrary — that is the whole point of
                            // this module — it just never fades on its own.)
                            if o.source == FORM_SOURCE {
                                o.cue.weight()
                            } else {
                                o.cue.weight() * decay(o.at, at, class.half_life_days())
                            }
                        })
                        .sum()
                };
                Facet {
                    class,
                    key: newest.key.clone(),
                    value: newest.value.clone(),
                    state: State::Provisional, // decided below, once values can be compared
                    score,
                    cue: strongest,
                    observations: obs.len(),
                    last_seen: newest.at,
                    evidence: obs
                        .iter()
                        .max_by(|a, b| a.at.total_cmp(&b.at))
                        .map(|o| o.evidence.clone())
                        .unwrap_or_default(),
                    pinned,
                    from_form: obs.iter().any(|o| o.source == FORM_SOURCE),
                }
            })
            .collect();

        // Strongest first, so the winner of each key is the one we meet first and
        // the budget below keeps the best rather than the earliest.
        facets.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.key.cmp(&b.key)));

        let mut won: Vec<(Class, String)> = Vec::new();
        let mut spent: HashMap<Class, usize> = HashMap::new();
        for f in facets.iter_mut() {
            let key = (f.class, f.key.clone());
            let contested = won.contains(&key);
            let used = spent.entry(f.class).or_insert(0);
            // A form field skips the evidence bar: it isn't evidence, it's the user
            // telling us directly. It still has to win its key and fit the budget.
            let earned = f.score >= ACTIVE_AT || f.from_form;
            if !contested && earned && *used < f.class.budget() {
                f.state = State::Active;
                *used += 1;
                won.push(key);
            }
        }
        facets
    }

    /// The Active facets only, grouped by class in a stable order.
    pub fn active_by_class(&self) -> Vec<(Class, Vec<Facet>)> {
        let all = self.facets();
        CLASSES
            .into_iter()
            .filter_map(|c| {
                let mine: Vec<Facet> =
                    all.iter().filter(|f| f.class == c && f.state == State::Active).cloned().collect();
                (!mine.is_empty()).then_some((c, mine))
            })
            .collect()
    }

    /// The block folded into a system prompt. Empty when there is nothing to say,
    /// so a fresh install costs no tokens.
    pub fn system_block(&self) -> String {
        let grouped = self.active_by_class();
        if grouped.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "\n\n[ABOUT THE USER — how they want to be worked with. Follow it without \
             mentioning it, and prefer what they say in this conversation over anything below.]",
        );
        for (class, facets) in grouped {
            out.push_str(&format!("\n{}:", class.heading()));
            for f in facets {
                out.push_str(&format!("\n- {}", f.value));
            }
        }
        out
    }

    /// Render the managed blocks into PROFILE.md, leaving anything the user wrote
    /// there by hand exactly where it was.
    pub fn write_profile_md(&self, vault: &Path) -> std::io::Result<PathBuf> {
        let path = profile_md_path(vault);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let existing = std::fs::read_to_string(&path).unwrap_or_else(|_| PROFILE_MD_HEADER.to_string());
        let rendered = render_managed(&existing, &self.active_by_class());
        std::fs::write(&path, &rendered)?;
        Ok(path)
    }
}
