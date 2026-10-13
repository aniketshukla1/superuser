//! Helpers for tests that read this crate's own source as text.
//!
//! A module split into a folder (`foo.rs` + `foo/*.rs`) is no longer one file, so
//! `include_str!("foo.rs")` sees only its `mod` lines. `module_source` reads the
//! whole module instead: every non-test file under `foo/`, then `foo.rs` last, so
//! code that cuts at the first `#[cfg(test)]` still keeps every handler.

use std::path::{Path, PathBuf};

/// The source of the module at `rel` (e.g. `"core/api_server"`, or `"lib"` for
/// the crate root), relative to `src/`. Leaked to `&'static str` so it drops in
/// wherever `include_str!` was.
pub fn module_source(rel: &str) -> &'static str {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // `foo.rs` + `foo/`, or a `foo/mod.rs` module whose split parts sit beside it.
    let flat = src.join(format!("{rel}.rs"));
    let root = if flat.exists() { flat } else { src.join(rel).join("mod.rs") };
    let dir = if rel == "lib" { None } else { Some(src.join(rel)) };
    let mut files = vec![];
    if let Some(d) = dir.filter(|d| d.is_dir()) {
        collect(&d, &mut files);
    }
    files.sort();
    files.push(root);
    files
        .iter()
        .map(|p| std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .collect::<Vec<_>>()
        .join("\n")
        .leak()
}

/// Test modules (`tests`, `*_tests`) are skipped: a test searching the source must
/// not find the string it is searching for in its own body.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        if stem == "tests" || stem.ends_with("_tests") {
            continue;
        }
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().map_or(false, |x| x == "rs") && stem != "mod" {
            out.push(p);
        }
    }
}

/// A GUI source file read as text, with the folder of the same name beside it when it
/// was split into one (`api.ts` + `api/*.ts`). `rel` is relative to `surfaces/gui/src`.
/// An error means the file is not there, as in a packaged build.
pub fn gui_source(rel: &str) -> std::io::Result<String> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../surfaces/gui/src").join(rel);
    let mut text = std::fs::read_to_string(&file)?;
    let folder = file.with_extension("");
    if folder.is_dir() {
        let mut parts = vec![];
        collect_any(&folder, &mut parts);
        parts.sort();
        for p in parts {
            text.push('\n');
            text.push_str(&std::fs::read_to_string(p)?);
        }
    }
    Ok(text)
}

/// Every non-test .ts/.tsx under `dir`.
fn collect_any(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.contains(".test.") || name.contains(".spec.") {
            continue;
        }
        if p.is_dir() {
            collect_any(&p, out);
        } else if name.ends_with(".ts") || name.ends_with(".tsx") {
            out.push(p);
        }
    }
}
