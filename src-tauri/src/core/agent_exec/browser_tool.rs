use super::*;

mod asks;
use asks::*;

/// The assistant's own browser — `core::agentbrowser`, one arm per tool.
pub(super) async fn browser_tool(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    use crate::core::agentbrowser::computer;

    if !ctx.settings.browser_enabled {
        return "error: the assistant's browser is switched off. The user can turn it on in Settings ▸ Browser; until then, use web_fetch to read a page.".into();
    }
    // Bug Reproduction works on staging and nowhere else — "Do not use production customer
    // data" — so opening, reading or acting on any other site is refused before the browser is
    // touched (`core::repro`).
    if ctx.persona == crate::core::repro::ASSISTANT {
        let page = match name {
            "browser_navigate" => Some(crate::core::browser::normalize_url(args["url"].as_str().unwrap_or(""))),
            _ => computer::current_url(&ctx.persona),
        };
        if let Err(e) = crate::core::repro::may_use(name, page.as_deref(), &ctx.settings.repro_staging) {
            return format!("error: {e}");
        }
    }
    // Acting on a page is an action like any other: a mode that never acts says no, and a
    // run nobody is watching may look but not touch — a wrong guess there acts unsupervised,
    // on sites the user is signed in to. Not parked for approval either: approving replays
    // the call later, against a page and refs that will have moved on.
    if matches!(name, "browser_click" | "browser_type" | "browser_key") {
        if matches!(ctx.mode.as_str(), "discuss" | "plan") {
            return format!(
                "Not done — this assistant's mode ({}) doesn't act on pages. Say what you would click or type instead.",
                ctx.mode
            );
        }
        if ctx.unattended {
            return "Not done — nobody is watching this run, so your browser can read pages but not click or type in them. \
                    Finish by saying what you would have done."
                .into();
        }
        if let Some(refusal) = browser_approval(ctx, name, args).await {
            return refusal;
        }
    }
    let c = match computer::get(&ctx.data_dir, &ctx.persona) {
        Ok(c) => c,
        Err(e) => return format!("error: {e}"),
    };
    let cancel = crate::commands::session_cancel_flag(&ctx.session_id);
    let s = |k: &str| args[k].as_str().unwrap_or("").trim().to_string();
    // Models send the id as a number or, often enough to matter, as "4".
    let snapshot_id = args["snapshot_id"].as_u64().or_else(|| args["snapshot_id"].as_str().and_then(|v| v.trim().parse().ok()));
    let need_id = || "error: give the snapshot_id from your latest browser_snapshot along with the ref.".to_string();

    match name {
        "browser_navigate" => {
            let url = crate::core::browser::normalize_url(&s("url"));
            match c.navigate(&url, Some(&cancel)).await {
                Ok(page) => {
                    // The page's words are a stranger's words, as with any fetched page.
                    ctx.taint();
                    // Where it landed counts, not only where it was sent: a staging link that
                    // redirects to production shows none of production to Bug Reproduction.
                    if ctx.persona == crate::core::repro::ASSISTANT {
                        if let Err(e) = crate::core::repro::landed(&page.url, &ctx.settings.repro_staging) {
                            return format!("error: {e}");
                        }
                    }
                    if let Some(frame) = c.keep_frame().await {
                        let item = serde_json::json!({
                            "kind": "computer", "persona": ctx.persona, "frame": frame,
                            "url": page.url, "title": page.title,
                        });
                        if let Some(tx) = &ctx.events {
                            let mut ev = item.clone();
                            ev["type"] = serde_json::json!("computer_frame");
                            let _ = tx.send(ev);
                        }
                        ctx.note_item(item);
                    }
                    let mut out = format!("Opened “{}” — {}\n\n", page.title, page.url);
                    out.push_str(&crate::core::webguard::wrap_untrusted(&page.url, &page.text));
                    if page.truncated {
                        out.push_str("\n\n(The page is longer than this. Scroll and read again for more.)");
                    }
                    out
                }
                Err(e) => browser_failed(e),
            }
        }
        "browser_console" => match c.console().await {
            Ok((env, notes)) => {
                // The page's own words, as much as anything it shows.
                ctx.taint();
                let mut out = format!("Browser: {env}\n");
                if notes.is_empty() {
                    out.push_str("Nothing reported since the last look — no console errors or warnings, no uncaught exceptions, no failed requests.");
                } else {
                    out.push_str(&format!("{} since the last look:\n", notes.len()));
                    let page = computer::current_url(&ctx.persona).unwrap_or_default();
                    out.push_str(&crate::core::webguard::wrap_untrusted(&page, &notes.join("\n")));
                }
                out
            }
            Err(e) => browser_failed(e),
        },
        "browser_screenshot" => match c.keep_frame().await {
            Some(frame) => {
                let url = computer::current_url(&ctx.persona).unwrap_or_default();
                let item = serde_json::json!({
                    "kind": "computer", "persona": ctx.persona, "frame": frame, "url": url, "title": s("why"),
                });
                if let Some(tx) = &ctx.events {
                    let mut ev = item.clone();
                    ev["type"] = serde_json::json!("computer_frame");
                    let _ = tx.send(ev);
                }
                ctx.note_item(item);
                format!("Kept a picture of {url}{}. It's in the conversation.", if s("why").is_empty() { String::new() } else { format!(" — {}", s("why")) })
            }
            None => "error: there's no page to picture yet — open one with browser_navigate first.".into(),
        },
        "browser_read" => match c.read().await {
            Ok(page) => {
                ctx.taint();
                let mut out = format!("“{}” — {}\n\n", page.title, page.url);
                out.push_str(&crate::core::webguard::wrap_untrusted(&page.url, &page.text));
                if page.truncated {
                    out.push_str("\n\n(The page is longer than this. Scroll and read again for more.)");
                }
                out
            }
            Err(e) => browser_failed(e),
        },
        "browser_snapshot" => match c.snapshot().await {
            Ok(list) => {
                // Labels are written by the page, so they taint exactly as its text does.
                ctx.taint();
                let url = computer::current_url(&ctx.persona).unwrap_or_default();
                crate::core::webguard::wrap_untrusted(&url, &list)
            }
            Err(e) => browser_failed(e),
        },
        "browser_click" => {
            let Some(id) = snapshot_id else { return need_id() };
            c.click(&s("ref"), id, Some(&cancel)).await.unwrap_or_else(browser_failed)
        }
        "browser_type" => {
            let Some(id) = snapshot_id else { return need_id() };
            let text = args["text"].as_str().unwrap_or("");
            c.type_text(&s("ref"), id, text, args["submit"].as_bool().unwrap_or(false), Some(&cancel))
                .await
                .unwrap_or_else(browser_failed)
        }
        "browser_key" => {
            let r = s("ref");
            let target = if r.is_empty() {
                None
            } else {
                match snapshot_id {
                    Some(id) => Some((r.as_str(), id)),
                    None => return need_id(),
                }
            };
            c.key(&s("key"), target).await.unwrap_or_else(browser_failed)
        }
        "browser_scroll" => {
            let down = s("direction") != "up";
            let pages = args["pages"].as_u64().unwrap_or(1) as u32;
            c.scroll(down, pages).await.unwrap_or_else(browser_failed)
        }
        "browser_request_help" => request_help(ctx, &c, &cancel, s("reason")).await,
        "browser_request_secret" => {
            let label = s("label");
            let Some(id) = snapshot_id else { return need_id() };
            request_secret(ctx, &c, &cancel, label, s("ref"), id).await
        }
        other => format!("unknown tool: {other}"),
    }
}
