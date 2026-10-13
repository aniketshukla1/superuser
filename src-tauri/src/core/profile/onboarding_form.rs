//! The onboarding form

use super::*;

/// What the user typed about themselves at onboarding, and can still edit.
///
/// Kept as its own struct rather than being dissolved into observations: it's a
/// form, the user expects to see the fields they filled in, and it is the one part
/// of the profile that is theirs to state rather than ours to infer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)] pub name: String,
    #[serde(default)] pub call_me: String, // preferred name / nickname
    #[serde(default)] pub role: String,
    #[serde(default)] pub about: String,
    #[serde(default)] pub tone: String, // e.g. "concise", "warm & detailed"
    #[serde(default)] pub interests: String,
    #[serde(default)] pub likes: String,
    #[serde(default)] pub dislikes: String,
    #[serde(default)] pub goals: String,
}

impl Profile {
    pub fn is_empty(&self) -> bool {
        [&self.name, &self.call_me, &self.role, &self.about, &self.tone, &self.interests, &self.likes, &self.dislikes, &self.goals]
            .iter()
            .all(|s| s.trim().is_empty())
    }
    pub(super) fn display_name(&self) -> &str {
        if !self.call_me.trim().is_empty() { self.call_me.trim() } else { self.name.trim() }
    }

    /// The form as observations, re-derived on every load rather than stored.
    ///
    /// Stamped `now` so they never decay: a form field is a standing declaration,
    /// not something that happened once. That also means editing the form takes
    /// effect immediately and leaves nothing stale behind — the old value simply
    /// stops being generated.
    pub(super) fn observations(&self) -> Vec<Observation> {
        let at = now();
        let mut out = Vec::new();
        // Every field is `Structural`: a form is data the user entered, not something
        // they said in the course of working. That ranks it below a statement made
        // outright — which is the behaviour you want, because a form filled in at
        // onboarding is exactly the thing most likely to be out of date.
        let mut push = |class: Class, key: &str, field: &str, value: &str| {
            let value = clean_value(value);
            if !value.is_empty() {
                out.push(Observation {
                    class,
                    key: key.into(),
                    value,
                    cue: Cue::Structural,
                    at,
                    // Names the field rather than repeating "from your profile",
                    // which the cue already says: this way the line tells you where
                    // to go and change it.
                    evidence: format!("Your profile ▸ {field}"),
                    source: FORM_SOURCE.into(),
                });
            }
        };
        let name = self.display_name().to_string();
        push(Class::Identity, "name", "Name", &name);
        push(Class::Identity, "role", "Role", &self.role);
        push(Class::Identity, "about", "About", &self.about);
        push(Class::Style, "tone", "Preferred tone", &self.tone);
        push(Class::Identity, "interests", "Interests", &self.interests);
        push(Class::Style, "likes", "Likes", &self.likes);
        push(Class::Veto, "dislikes", "Never do", &self.dislikes);
        push(Class::Goal, "goals", "Goals", &self.goals);
        out
    }
}

/// Marks an observation as coming from the form, so it is never persisted into the
/// observation log (it's regenerated) and can be labelled in the UI.
pub const FORM_SOURCE: &str = "form";

pub(super) fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("profile.json")
}

pub fn load(data_dir: &Path) -> Profile {
    std::fs::read_to_string(store_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, p: &Profile) {
    let _ = crate::core::store::write_json_atomic(&store_path(data_dir), p);
}

/// Markdown body for the durable "User Profile" memory.
pub fn memory_body(p: &Profile) -> String {
    let field = |label: &str, v: &str| if v.trim().is_empty() { String::new() } else { format!("- **{label}:** {}\n", v.trim()) };
    format!(
        "{}{}{}{}{}{}{}{}",
        field("Name", &p.name),
        field("Prefers to be called", &p.call_me),
        field("Role / work", &p.role),
        field("About", &p.about),
        field("Preferred tone", &p.tone),
        field("Interests", &p.interests),
        field("Likes", &p.likes),
        field("Dislikes", &p.dislikes),
    ) + &field("Goals", &p.goals)
}
