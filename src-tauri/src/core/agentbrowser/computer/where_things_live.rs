//! where things live

use super::*;

/// A persona id becomes a directory that `reset` deletes, so only a plain one is accepted.
pub fn is_plain_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && id.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn root(data_dir: &Path, persona: &str) -> PathBuf {
    data_dir.join("computers").join(persona)
}

pub(super) fn profile_dir(data_dir: &Path, persona: &str) -> PathBuf {
    root(data_dir, persona).join("profile")
}

pub fn frames_dir(data_dir: &Path, persona: &str) -> PathBuf {
    root(data_dir, persona).join("frames")
}

/// Is `id` a frame name this module could have written? Checked before a path is built
/// from it, because it arrives in a URL.
pub fn is_frame_id(id: &str) -> bool {
    id.len() <= 64 && id.starts_with("f-") && id[2..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}
