use super::super::*;

/// Rename the product's old word inside assistants the user already made.
///
/// The app called them "coworkers" and now calls them assistants. Shipped copy
/// moved with the rename; assistants saved BEFORE it did not, because they are
/// files on disk holding text generated at creation time — so a list of built-ins
/// reading "Assistant" sat above user-made ones reading "Your Finance coworker".
///
/// Whole-word and case-preserving, the same substitution the source got. It runs
/// on load and is idempotent: once no file contains the word there is nothing to
/// rewrite, so no marker file is needed to remember it happened.
///
/// This edits the user's own content, which is not something to do lightly. It is
/// justified because the word is the product's name for the thing, not the user's
/// prose about it — they wrote "Finance", the app wrote "Your Finance coworker".
pub(in crate::core::personas) fn migrate_wording(data_dir: &Path) {
    let dir = data_dir.join("personas");
    let Ok(rd) = fs::read_dir(&dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
        if !text.contains("oworker") {
            continue;
        }
        let renamed = text
            .replace("Coworkers", "Assistants")
            .replace("coworkers", "assistants")
            .replace("Coworker", "Assistant")
            .replace("coworker", "assistant");
        let _ = crate::core::store::write_atomic(&path, &renamed);
    }
}

pub(in crate::core::personas) fn load_state(data_dir: &Path) -> PersonaState {
    fs::read_to_string(data_dir.join("personas.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
