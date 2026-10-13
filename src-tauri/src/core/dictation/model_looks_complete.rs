use super::*;

/// Is the file on disk big enough to be the real model?
pub fn model_looks_complete(data_dir: &Path) -> bool {
    std::fs::metadata(model_path(data_dir)).map(|m| m.len() >= MIN_PLAUSIBLE_BYTES).unwrap_or(false)
}
