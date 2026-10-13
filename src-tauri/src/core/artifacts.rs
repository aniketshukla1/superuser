//! Big results are handed on by path, not by value.
//!
//! `core::context` already shrinks oversized tool output — but only by cutting the
//! middle out of it, and only for steps the model has already moved past. Two
//! things go wrong on a long run. Truncation is **lossy with no way back**: once
//! the middle is gone the model can never ask for it. And it shrinks one payload
//! at a time, which does nothing about the *aggregate* — twenty results of 19k
//! chars each are individually under the limit and collectively fatal.
//!
//! So anything genuinely large gets written to `outputs/` in the run's own folder,
//! and what goes into the transcript is a short abstract plus the path. The model
//! sees roughly what it got, knows where the rest is, and can read any part of it
//! back with `read_file`. Nothing is lost; it just has to ask.
//!
//! Truncation stays exactly as it is. It is the fallback for everything this
//! misses, and for every failure here — a full disk, a read-only folder — the
//! caller keeps its inline text and falls through to it.

use std::path::{Path, PathBuf};

/// Where deliverables live inside a run's folder. A plain subdirectory, so it
/// shows up in the artifacts panel like anything else the run produced.
pub const OUTPUTS_DIR: &str = "outputs";

/// Results longer than this are worth a file. Well under the 20k truncation limit,
/// so the interesting cases are offloaded rather than cut.
pub const OFFLOAD_THRESHOLD: usize = 12_000;

/// How much of the head stays in the transcript. Enough to see what the thing is
/// and whether it's the right thing, not enough to be worth keeping in full.
const ABSTRACT_CHARS: usize = 700;

/// Turn a label into a filename component. Fail-closed by construction: only
/// `[a-z0-9-]` survives, so no separator, traversal or absolute path can be
/// smuggled in through a tool name or a subagent's task text.
fn slug(label: &str) -> String {
    let s: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    let s: String = s.chars().take(48).collect();
    // Collapse runs of dashes so "run shell: ls -la" doesn't become a hyphen field.
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c == '-' {
            if !last_dash {
                out.push(c);
            }
            last_dash = true;
        } else {
            out.push(c);
            last_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "result".into() } else { out }
}

/// A path under `root/outputs` that doesn't collide with an existing file.
fn free_path(root: &Path, label: &str) -> PathBuf {
    let dir = root.join(OUTPUTS_DIR);
    let base = slug(label);
    for n in 0..1000 {
        let name = if n == 0 { format!("{base}.txt") } else { format!("{base}-{n}.txt") };
        let p = dir.join(&name);
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("{base}-{}.txt", std::process::id()))
}

/// Write `text` to the run's `outputs/` and return what the model should see
/// instead: an abstract, the path, and the size.
///
/// `None` means "keep what you had" — the text was small enough, the write failed,
/// or the replacement wouldn't actually be shorter. Callers treat `None` as "carry
/// on unchanged", so this can never make a result bigger or lose one.
pub fn offload(root: &Path, label: &str, text: &str) -> Option<String> {
    if text.len() <= OFFLOAD_THRESHOLD {
        return None;
    }
    let path = free_path(root, label);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok()?;
    }
    std::fs::write(&path, text).ok()?;

    let rel = path
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
    let head: String = text.chars().take(ABSTRACT_CHARS).collect();
    let replacement = format!(
        "{head}\n\n[…{} more characters. The full result is saved at `{rel}` — read it with read_file, \
         which takes a line range if you only need part of it.]",
        text.len().saturating_sub(head.len())
    );

    // Never grow the payload. If the pointer costs more than the thing it points
    // at, the file is still written (it's a deliverable either way) but the model
    // keeps the original.
    if replacement.len() >= text.len() {
        return None;
    }
    Some(replacement)
}

/// The line in the system prompt that makes the convention legible to the model.
/// Without it, an agent that receives a pointer has no reason to believe reading
/// the path will work.
pub fn prompt_note() -> String {
    format!(
        "\n\nLarge results are saved to `{OUTPUTS_DIR}/` in your working folder and replaced in this \
         conversation by a short extract plus the file path. That file is the complete version — \
         read it with read_file when you need more than the extract shows. Write your own long \
         deliverables there too, and refer to them by path rather than pasting them back."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("su-art-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn small_results_are_left_alone() {
        let root = scratch();
        assert!(offload(&root, "run_shell", "a short answer").is_none());
        assert!(!root.join(OUTPUTS_DIR).exists(), "no file for something that didn't need one");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_big_result_becomes_an_abstract_and_a_path_and_loses_nothing() {
        let root = scratch();
        let big = "x".repeat(OFFLOAD_THRESHOLD + 5_000);
        let out = offload(&root, "run_shell", &big).expect("should offload");

        assert!(out.len() < big.len(), "the transcript gets smaller");
        assert!(out.contains("outputs/run-shell.txt"), "names the path: {out}");
        assert!(out.contains("read_file"), "says how to get the rest");

        // The whole thing is on disk, byte for byte.
        let saved = std::fs::read_to_string(root.join(OUTPUTS_DIR).join("run-shell.txt")).unwrap();
        assert_eq!(saved, big, "offloading is not truncation — nothing is lost");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn two_results_from_the_same_tool_do_not_overwrite_each_other() {
        let root = scratch();
        let a = "a".repeat(OFFLOAD_THRESHOLD + 1);
        let b = "b".repeat(OFFLOAD_THRESHOLD + 1);
        let first = offload(&root, "grep", &a).unwrap();
        let second = offload(&root, "grep", &b).unwrap();
        assert!(first.contains("outputs/grep.txt"));
        assert!(second.contains("outputs/grep-1.txt"), "got: {second}");
        assert_eq!(std::fs::read_to_string(root.join("outputs/grep.txt")).unwrap(), a);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_label_cannot_escape_the_outputs_folder() {
        // The label comes from a tool name or a subagent's task text, so it is not
        // trusted input. Everything that isn't alphanumeric becomes a dash.
        let root = scratch();
        let big = "x".repeat(OFFLOAD_THRESHOLD + 1);
        offload(&root, "../../etc/passwd", &big).unwrap();
        assert!(root.join("outputs/etc-passwd.txt").exists());
        assert!(!root.parent().unwrap().join("etc").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn slugs_are_readable_rather_than_a_field_of_dashes() {
        assert_eq!(slug("run_shell: ls -la /tmp"), "run-shell-ls-la-tmp");
        assert_eq!(slug("!!!"), "result");
        assert_eq!(slug(""), "result");
    }
}
