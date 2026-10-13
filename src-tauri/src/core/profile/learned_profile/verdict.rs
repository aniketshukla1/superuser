use super::super::*;

/// A facet the user has pinned (shielded from decay) or forgotten (blocked from
/// coming back). Both are keyed on class+key+value, so forgetting one belief about
/// a key leaves the others free to win it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub class: Class,
    pub key: String,
    pub value: String,
    pub at: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(in crate::core::profile) struct Persisted {
    #[serde(default)]
    pub(in crate::core::profile) observations: Vec<Observation>,
    #[serde(default)]
    pub(in crate::core::profile) pinned: Vec<Verdict>,
    #[serde(default)]
    pub(in crate::core::profile) forgotten: Vec<Verdict>,
}

/// Everything learned about how to work with this person, plus the user's own
/// verdicts on it.
pub struct Learned {
    pub(in crate::core::profile) data_dir: PathBuf,
    pub(in crate::core::profile) inner: Persisted,
    /// The form, folded in on load so callers never have to remember to.
    pub(in crate::core::profile) form: Profile,
}

pub(in crate::core::profile) fn facets_path(data_dir: &Path) -> PathBuf {
    data_dir.join("profile-facets.json")
}
