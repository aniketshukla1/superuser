use super::super::*;

/// Review one conversation and apply whatever it justifies. Returns `Silent` when
/// nothing changed on disk — regardless of what the model said it did.
pub async fn review(
    settings: &Settings,
    data_dir: &Path,
    vault: &Path,
    transcript: &str,
) -> Outcome {
    use crate::core::model_router::{self, ChatMessage};

    if transcript.trim().is_empty() {
        return Outcome::Silent;
    }
    let scope = Scope::for_review(data_dir, vault);
    let snap = snapshot(&scope);

    let msgs = vec![
        ChatMessage { role: "system".into(), content: REVIEW_SYSTEM.into(), images: vec![], docs: vec![] },
        ChatMessage { role: "user".into(), content: build_prompt(transcript, data_dir, vault), images: vec![], docs: vec![] },
    ];
    let raw = match model_router::complete(settings, &msgs, model_router::Workload::BackgroundCapable).await {
        Ok((s, _)) => s,
        Err(e) => return Outcome::Failed(e),
    };

    let Some(review) = parse_review(&raw) else {
        // A silent pass is the normal, desired outcome — but a model that simply
        // ignored the output format looks identical from the outside, and that is how
        // this feature ends up quietly never working. Small local models (~3B) tend
        // to echo the prompt back rather than emit the edit blocks; measured on
        // llama3.2:3b, which reasons about the fix correctly and then never formats
        // it. Say so, instead of reporting "nothing to learn" forever.
        if !raw.contains(SILENT_TOKEN) {
            eprintln!(
                "[superuser] evolution: '{}' answered without a usable edit or <silent/>. \
                 Reviews need a model that follows an output format (7B+ locally, or a cloud model). First 200 chars: {}",
                settings.ollama_model,
                raw.chars().take(200).collect::<String>().replace('\n', " "),
            );
        }
        return Outcome::Silent;
    };
    let Review { edits, patterns, because, note } = review;

    // The wiki is written first, and unconditionally.
    //
    // Ordering matters: a pattern must survive an edit that a guard then refuses, an
    // undo the user performs a minute later, and a pass that had no edit to make at
    // all. Those are the cases where the old loop lost everything it had noticed.
    let noted = wiki::commit(
        vault,
        &patterns.iter().map(|p| (p.id.clone(), p.title.clone(), p.kind, p.body.clone())).collect::<Vec<_>>(),
        &[],
        &if patterns.is_empty() {
            String::new()
        } else {
            format!("noted {}", patterns.iter().map(|p| p.id.as_str()).collect::<Vec<_>>().join(", "))
        },
    );
    // What motivated this pass: what it just wrote, plus what it cited from before.
    // A citation of a pattern that does not exist is dropped rather than recorded —
    // it would otherwise become a PURPOSE.md link to a missing file, which reads as
    // a lost page rather than as the invented reference it is.
    let known = Wiki::new(vault);
    let mut motives = noted.clone();
    for b in &because {
        if !motives.contains(b) && known.read_pattern(b).is_some() {
            motives.push(b.clone());
        }
    }

    for e in &edits {
        // A refused edit skips that file and keeps the rest; a review that wandered
        // outside the workspace should not also lose a legitimate skill fix.
        let Ok(full) = resolve_path(data_dir, vault, &e.path) else { continue };
        let guard = if e.path == "journal/MEMORY.md" {
            guard_core_edit(&std::fs::read_to_string(&full).unwrap_or_default(), &e.content)
        } else {
            let name = full.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            guard_skill_edit(&name, &e.content)
        };
        if guard.is_err() {
            continue;
        }
        if let Some(dir) = full.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = crate::core::store::write_atomic(&full, &(e.content.trim_end().to_string() + "\n"));
    }

    // The gate: what actually changed on disk, not what the answer claimed.
    // Computed before PURPOSE.md is written, so the count the user is shown is the
    // files they care about rather than the bookkeeping beside them.
    let changed = changed_since(&snap, &scope);
    if changed.is_empty() {
        // Nothing the user can see — but the pass is no longer wasted if it noticed
        // something, which is the whole point of the layer.
        return if noted.is_empty() { Outcome::Silent } else { Outcome::Noted { patterns: noted } };
    }
    save_snapshot(data_dir, &snap);
    let summary = if note.trim().is_empty() {
        format!("I updated {} file{} from what we just went through.", changed.len(), if changed.len() == 1 { "" } else { "s" })
    } else {
        note
    };

    // Why each skill now looks the way it does, and a ledger row per file so that an
    // undo has something to flip. The rows outlive the undo; the PURPOSE files do
    // not, because they sit under `skills/` and roll back with what they explain.
    let wiki_store = Wiki::new(vault);
    for path in &changed {
        let p = Path::new(path);
        if p.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
            if let (Some(dir), Some(name)) = (p.parent(), p.parent().and_then(|d| d.file_name())) {
                let _ = wiki::write_purpose(dir, &name.to_string_lossy(), &motives, &summary);
            }
        }
        let _ = wiki_store.record_impact(&wiki::Impact {
            at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            target: target_name(p),
            patterns: motives.clone(),
            verdict: wiki::Verdict::Accepted,
            snapshot: snap.id.clone(),
            note: summary.clone(),
        });
    }
    let _ = wiki_store.log(&format!("changed {} — {}", changed.iter().map(|c| target_name(Path::new(c))).collect::<Vec<_>>().join(", "), summary));

    let _ = record(vault, &summary, &changed);
    Outcome::Changed { summary, files: changed, snapshot: snap.id }
}

/// How a changed file is named in the ledger: a skill by its own name, anything
/// else by its filename. `skills/release/SKILL.md` reads as `release`, because
/// "SKILL.md" in a column of other "SKILL.md"s identifies nothing.
pub(in crate::core::evolution) fn target_name(p: &Path) -> String {
    let file = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if file == "SKILL.md" || file == "PURPOSE.md" {
        return p.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or(file);
    }
    file
}

/// Append a dated record of the pass, next to the dream diaries.
pub(in crate::core::evolution) fn record(vault: &Path, summary: &str, files: &[String]) -> std::io::Result<()> {
    let journal = Journal::new(vault);
    let dir = journal.root.join("evolution");
    std::fs::create_dir_all(&dir)?;
    let date = crate::core::journal::today();
    let path = dir.join(format!("{date}.md"));
    let stamp = chrono::Local::now().format("%H:%M").to_string();
    let short: Vec<String> = files
        .iter()
        .map(|f| Path::new(f).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| f.clone()))
        .collect();
    let existing = std::fs::read_to_string(&path).unwrap_or_else(|_| format!("# Self-evolution — {date}\n"));
    crate::core::store::write_atomic(&path, &format!("{existing}\n## {stamp}\n\n{summary}\n\nChanged: {}\n", short.join(", ")))
}
