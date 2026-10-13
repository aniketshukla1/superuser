use super::super::*;

/// `POST /v1/personas` — create an assistant from the New-assistant modal.
///
/// Writes a manifest to `data_dir/personas/<id>.md`, the same format built-ins use,
/// so a created persona is a plain file the user can read, edit or delete by hand.
/// The intro is derived from what they typed rather than invented — no model call,
/// nothing to hallucinate, and it always matches the responsibilities they wrote.
pub(in crate::core::api_server) async fn persona_create(AxState(ctx): AxState<Arc<ApiContext>>, Json(body): Json<Value>) -> Json<Value> {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();

    let name = body["name"].as_str().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Json(json!({ "ok": false, "error": "give the assistant a name" }));
    }
    let duties = body["responsibilities"].as_str().unwrap_or("").trim().to_string();
    let icon = body["icon"].as_str().filter(|s| !s.is_empty()).unwrap_or("sparkles").to_string();

    // id: a filesystem- and url-safe slug of the name.
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if base.is_empty() {
        return Json(json!({ "ok": false, "error": "that name has no usable letters or digits" }));
    }
    let registry = crate::core::personas::PersonaRegistry::load(&dir);
    // A name whose slug is already taken is refused, and the refusal says WHO has it.
    //
    // This used to append a number — "Finance" became `finance-2` — and say nothing.
    // The audit log shows what that produces: one person creating "Finance" four
    // times in a row, getting `finance`, `finance-2`, `finance-2`, `finance-3`, and
    // reasonably concluding it did not work. Nothing on screen ever mentioned that
    // there was already a Finance Analyst, or that the thing they got was not named
    // what they typed. An id you did not choose and were not told about is worse
    // than being asked for another name.
    if let Some(existing) = registry.manifest(&base) {
        return Json(json!({
            "ok": false,
            "error": format!(
                "There's already an assistant called {} using that name. Pick a different one.",
                if existing.name.trim().is_empty() { base.clone() } else { existing.name.clone() }
            )
        }));
    }
    let id = base.clone();

    let tagline = if duties.is_empty() { format!("Your {name} assistant") } else { duties.lines().next().unwrap_or(&duties).trim().to_string() };
    let intro = if duties.is_empty() {
        format!("I'm {name}. What would you like me to take on?")
    } else {
        format!("I'm {name}. {duties} What would you like to start with?")
    };
    let starters: Vec<String> = body["starters"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    // Permission mode decides whether it acts, asks, or only plans. Unknown values
    // fall back to "interactive" (ask before anything consequential) — the safe end.
    let mode = match body["permission_mode"].as_str().unwrap_or("interactive") {
        m @ ("discuss" | "plan" | "interactive" | "auto") => m,
        _ => "interactive",
    }
    .to_string();

    // Tools it may use. Empty selection = the standard set, so a user who ignores
    // the field still gets a working assistant.
    let picked: Vec<String> = body["tools"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(str::to_string).collect())
        .unwrap_or_default();
    // One source of truth, so the picker cannot offer something this drops on the
    // way in. It used to be a private list of six against a catalogue of fifteen.
    use crate::core::personas::GRANTABLE_TOOLS;
    let tools: Vec<String> = if picked.is_empty() {
        ["recall_memory", "web", "files", "todo", "connectors"].iter().map(|s| s.to_string()).collect()
    } else {
        picked.into_iter().filter(|t| GRANTABLE_TOOLS.contains(&t.as_str())).collect()
    };

    // Reference files: written beside the manifest so the assistant can read them
    // with its `files` tool. Text only for now — a persona can't do anything useful
    // with raw bytes, and silently storing an unreadable blob would be a lie.
    let mut saved_files: Vec<String> = Vec::new();

    // YAML-safe: quote and escape the free-text fields.
    let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let mut fm = String::new();
    fm.push_str("---\n");
    fm.push_str(&format!("id: {id}\n"));
    fm.push_str(&format!("name: {}\n", q(&name)));
    fm.push_str(&format!("icon: {icon}\n"));
    fm.push_str(&format!("tagline: {}\n", q(&tagline)));
    fm.push_str("family: knowledge\n");
    fm.push_str(&format!("default_permission_mode: {mode}\n"));
    fm.push_str(&format!("tools: [{}]\n", tools.join(", ")));
    fm.push_str("messaging: true\nconnectors: true\n");
    fm.push_str(&format!("description: {}\n", q(&tagline)));
    fm.push_str(&format!("intro: {}\n", q(&intro)));
    if !starters.is_empty() {
        fm.push_str("starters:\n");
        for s in &starters {
            fm.push_str(&format!("  - {}\n", q(s)));
        }
    }
    fm.push_str("---\n");
    fm.push_str(&format!("You are {name}, the user's assistant.\n\n"));
    if !duties.is_empty() {
        fm.push_str(&format!("Your responsibilities: {duties}\n\n"));
    }
    fm.push_str(
        "Work like this:\n\
         - Start any multi-step task with a short todo plan (2-4 items) and keep exactly one item in progress.\n\
         - Ground your work in what you know about the user; recall relevant memories before you start.\n\
         - Produce an actual artifact and say where it lives.\n\
         - Ask before anything that sends, publishes or deletes.\n",
    );

    let pdir = dir.join("personas");
    if let Err(e) = std::fs::create_dir_all(&pdir) {
        return Json(json!({ "ok": false, "error": format!("couldn't create the personas folder: {e}") }));
    }
    // Reference files live in <id>-files/ beside the manifest.
    if let Some(arr) = body["files"].as_array() {
        let fdir = pdir.join(format!("{id}-files"));
        let _ = std::fs::create_dir_all(&fdir);
        for f in arr {
            let (Some(fname), Some(content)) = (f["name"].as_str(), f["content"].as_str()) else { continue };
            // Basename only — a crafted name must not escape the folder.
            let safe = std::path::Path::new(fname)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file.txt")
                .to_string();
            if safe.is_empty() || content.is_empty() {
                continue;
            }
            if std::fs::write(fdir.join(&safe), content).is_ok() {
                saved_files.push(safe);
            }
        }
    }
    if !saved_files.is_empty() {
        fm.push_str(&format!(
            "\nReference material you have been given lives in `{}`. Read it with your files tool before answering questions it covers: {}.\n",
            pdir.join(format!("{id}-files")).to_string_lossy(),
            saved_files.join(", ")
        ));
    }

    let path = pdir.join(format!("{id}.md"));
    if let Err(e) = std::fs::write(&path, &fm) {
        return Json(json!({ "ok": false, "error": format!("couldn't write the manifest: {e}") }));
    }
    // Parse it back: if we just wrote something unloadable, say so now rather than
    // leaving a file that breaks the registry on next load.
    match crate::core::personas::parse_manifest(&fm, Some(&id), false, Some(path.to_string_lossy().into_owned())) {
        Ok(_) => {
            // Turn it on. `is_enabled` defaults user personas to off — right for one that
            // ARRIVED (installed from a repo, a gallery, a folder), where the user has not
            // yet read what it does. It is wrong for one the user just wrote in a modal and
            // pressed Create on: filling in the form IS the opt-in.
            //
            // Left off, a brand-new assistant was invisible in its own product. It never
            // appeared in "Start a session as"; the client's `personaOf` could not resolve
            // it, so the header showed the raw slug ("repro-shell") instead of the name; and
            // the session the create flow drops you into belonged to a persona the sidebar
            // filters out — so on the next launch the app resumed the last VISIBLE thing
            // instead, which is how creating an assistant landed you back in the Coder.
            //
            // Reloaded rather than reusing the registry from the id-collision check above:
            // that one was loaded before this manifest existed, and `set_enabled` refuses an
            // id it has no entry for.
            let mut reg = crate::core::personas::PersonaRegistry::load(&dir);
            let _ = reg.set_enabled(&id, true);
            crate::core::audit::log(&dir, "persona", "user", &id, &format!("created {name}"), true);
            Json(json!({ "ok": true, "persona": { "id": id, "name": name, "icon": icon, "tagline": tagline, "intro": intro, "starters": starters, "tools": tools, "permission_mode": mode, "files": saved_files } }))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Json(json!({ "ok": false, "error": format!("the manifest didn't parse: {e}") }))
        }
    }
}
