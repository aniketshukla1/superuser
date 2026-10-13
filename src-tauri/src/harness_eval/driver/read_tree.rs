use super::*;

    /// Which app instance to measure.
    ///
    /// `HARNESS_EVAL_PORT` exists because two things want port 8765: the user's
    /// `tauri dev` session and a headless instance for a long sweep. `tauri dev`
    /// wins — it SIGTERMs whatever holds the port and starts its own app — and that
    /// killed four consecutive five-hour matrices. Every post-mortem looked like an
    /// app bug (silent death, no panic, no crash report) and every one of them was
    /// really "someone opened the app".
    ///
    /// The app already falls back to an OS-assigned port when 8765 is taken and
    /// writes `sidecar-<port>.token` beside it, so a second instance costs nothing.
    /// Pointing the driver at that one lets a measurement run for hours while the
    /// user keeps working in theirs.
    pub(super) fn port() -> u16 {
        std::env::var("HARNESS_EVAL_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8765)
    }

    pub(super) fn api(path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", port())
    }

    /// How long one task may take before the driver gives up.
    ///
    /// Generous by default and configurable, because the ceiling is set by the
    /// slowest model anyone points this at: `gemma4:12b` spent 108 seconds on a
    /// single step of a six-step task, and the first version's 600s deadline scored
    /// that as the model failing the task.
    pub(super) fn deadline_secs() -> u64 {
        std::env::var("HARNESS_EVAL_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2_700)
    }

    /// The sidecar token of the instance under test.
    ///
    /// `SUPERUSER_DATA_DIR` first, and it has to be: the app writes its token beside
    /// the rest of its data, so an instance moved to a scratch directory puts the
    /// token there too. Reading the default location regardless is how the first run
    /// of the brain group failed — the driver sent the *real* app's stale token to
    /// the scratch instance, got a 401, and reported it as "no data dir in
    /// /v1/settings" three layers downstream.
    pub(super) fn token() -> Option<String> {
        let dir = match std::env::var("SUPERUSER_DATA_DIR") {
            Ok(d) if !d.trim().is_empty() => PathBuf::from(d),
            _ => PathBuf::from(std::env::var("HOME").ok()?).join("Library/Application Support/ai.superuser.app"),
        };
        std::fs::read_to_string(dir.join(format!("sidecar-{}.token", port()))).ok().map(|s| s.trim().to_string())
    }

    /// Wait for the app to answer again, up to `secs`.
    ///
    /// A five-hour sweep outliving a restart is worth a few lines. Measured the hard
    /// way: the app exited cleanly partway through run 1 — no crash report, no OOM,
    /// which is what closing a Tauri window looks like — and the driver spent the
    /// next 43 tasks firing at a dead port and recording blanks. The blanks were
    /// correct (they are "not measured", never failures), and a report full of them
    /// still looked enough like a result to need reading twice.
    pub(super) async fn wait_for_app(http: &crate::core::egress::Client, tok: &str, secs: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
        while std::time::Instant::now() < deadline {
            if settings(http, tok).await.is_some() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        false
    }

    pub(super) async fn settings(http: &crate::core::egress::Client, tok: &str) -> Option<Value> {
        let r = http.get(api("/v1/settings")).header("x-superuser-token", tok).send().await.ok()?;
        // A refused request is not settings. Without this the 401 body parses as
        // perfectly good JSON, every field the driver wants is absent, and the run
        // reports twenty-six confusing task failures instead of one clear one.
        if !r.status().is_success() {
            eprintln!("/v1/settings refused the token: {}", r.status());
            return None;
        }
        r.json().await.ok()
    }

    /// Where a session with no record does its work — `legacy_session_dir`, derived
    /// from the one absolute path the settings payload exposes.
    pub(super) fn work_dir(settings: &Value, sid: &str) -> Option<PathBuf> {
        let secrets = settings["secrets_path"].as_str()?;
        let data_dir = PathBuf::from(secrets).parent()?.to_path_buf();
        Some(data_dir.join("sessions").join(sid))
    }

    /// The vault a brain task may seed, or `None` if this run must not touch one.
    ///
    /// Seeding a vault means **deleting what is in it first**, or the previous
    /// task's memories answer this one's question. That is a destructive operation
    /// pointed at whatever app happens to be listening on the port, and the port is
    /// 8765 by default — the user's own app.
    ///
    /// So the guard is deliberately awkward: the operator must set
    /// `SUPERUSER_DATA_DIR` for the driver, the app under test must report *that
    /// same directory*, and both are compared after `canonicalize` so a symlink or a
    /// trailing slash cannot make two different directories look like one. Anything
    /// less than an exact match returns `None` and the brain tasks refuse to run.
    /// A skipped task is a hole in the table; a wiped vault is someone's notes.
    pub(super) fn scratch_vault(settings: &Value) -> Option<PathBuf> {
        let want = std::env::var("SUPERUSER_DATA_DIR").ok().filter(|s| !s.trim().is_empty())?;
        let secrets = settings["secrets_path"].as_str()?;
        let live = PathBuf::from(secrets).parent()?.to_path_buf();
        let real = |p: &PathBuf| std::fs::canonicalize(p).ok();
        (real(&PathBuf::from(&want))? == real(&live)?).then(|| live.join("vault"))
    }

    /// Replace the scratch vault's contents with this task's memories.
    ///
    /// Only `*.md` at the vault root, which is exactly what `MemoryEngine::list`
    /// reads — the journal lives in `vault/journal/` and is left alone, so a brain
    /// task cannot silently destroy the consolidation history it may later want to
    /// grade.
    pub(super) fn seed_vault(vault: &PathBuf, task: &Task, marker: &str) -> Result<(), String> {
        std::fs::create_dir_all(vault).map_err(|e| format!("vault dir: {e}"))?;
        if let Ok(entries) = std::fs::read_dir(vault) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "md") {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        for (name, body) in task.vault {
            std::fs::write(vault.join(name), body.replace(MARKER_SLOT, marker))
                .map_err(|e| format!("vault fixture {name}: {e}"))?;
        }
        Ok(())
    }

    /// Everything under `dir`, as relative path → contents. Binary and oversized
    /// files are skipped rather than lossily decoded: a check that matched on
    /// replacement characters would be worse than one that found nothing.
    pub(super) fn read_tree(dir: &PathBuf) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                if p.metadata().map(|m| m.len() > 256 * 1024).unwrap_or(true) {
                    continue;
                }
                if let (Ok(text), Ok(rel)) = (std::fs::read_to_string(&p), p.strip_prefix(dir)) {
                    out.insert(rel.to_string_lossy().to_string(), text);
                }
            }
        }
        out
    }

    /// One judged line on one failing turn: `SUPPORTED`, or `UNSUPPORTED: <claim>`.
    /// Whether a judge on `model` may run at all under the user's switch.
    ///
    /// `model_router::complete_on` dispatches on the model id and does **not** consult
    /// `local_only` — only `route` does — so a judge built on it would bypass the
    /// switch without a word. It is enforced here instead, strictly: an Ollama tag is
    /// local, and anything a provider claims is treated as cloud. A custom endpoint on
    /// loopback is refused too; that is the right way for a privacy switch to be wrong.
    pub(super) fn judge_allowed(model: &str, local_only: bool) -> Result<(), String> {
        let local = match crate::core::providers::resolve(model) {
            None => true,
            Some(p) => matches!(p.kind, crate::core::providers::ProviderKind::Ollama),
        };
        if local_only && !local {
            return Err(format!("refused: local-only mode is on and {model} is not a local model"));
        }
        Ok(())
    }
