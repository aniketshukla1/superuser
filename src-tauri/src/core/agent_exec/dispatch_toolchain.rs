use super::*;

/// The toolchain tools: what is installed, and what each check actually did.
///
/// `check_tool` writes the ledger itself rather than reporting and trusting the
/// model to disclose it later. That is the whole design. A security pass asked to
/// sweep git history, on a machine without the tool that does it, used to produce
/// a clean report — the check did not fail, it disappeared, and "we could not
/// look" was read as "there is nothing there". Recording the gap at the moment of
/// discovery means the model can amend the entry but cannot lose it.
pub(super) async fn dispatch_toolchain(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> Option<String> {
    use crate::core::toolchain::{self, Presence, Status};

    let named = || args["name"].as_str().unwrap_or("").trim().to_string();
    match name {
        "check_tool" => {
            let n = named();
            let Some(t) = toolchain::find(&n) else {
                return Some(format!(
                    "error: `{n}` isn't a scanner this app knows about. Known: {}. \
                     For anything else, check for it with `run_shell` and install it the normal way.",
                    toolchain::names()
                ));
            };
            Some(match toolchain::probe(t) {
                Presence::Present { version } => {
                    toolchain::record(
                        &mut ctx.coverage.lock().unwrap(),
                        t.check,
                        Status::Ran { tool: t.name.into(), version: version.clone() },
                    );
                    format!("{} is installed: {version}", t.name)
                }
                Presence::Missing => {
                    toolchain::record(
                        &mut ctx.coverage.lock().unwrap(),
                        t.check,
                        Status::Skipped { reason: format!("{} is not installed on this machine", t.name) },
                    );
                    format!(
                        "{} is NOT installed, so \"{}\" has been recorded as NOT RUN.\n\n\
                         Without it: {}.\n\n\
                         Three ways forward, in order: ask for it with `request_tool`; do the equivalent by hand \
                         and then `record_coverage` this check as `degraded`; or leave it skipped and say so \
                         plainly in your report. What you must not do is carry on as though the check passed.",
                        t.name, t.check, t.fallback
                    )
                }
            })
        }
        "request_tool" => {
            let n = named();
            let reason = args["reason"].as_str().unwrap_or("").trim();
            let Some(t) = toolchain::find(&n) else {
                return Some(format!("error: `{n}` isn't in the install catalogue. Known: {}", toolchain::names()));
            };
            if let Presence::Present { version } = toolchain::probe(t) {
                toolchain::record(
                    &mut ctx.coverage.lock().unwrap(),
                    t.check,
                    Status::Ran { tool: t.name.into(), version: version.clone() },
                );
                return Some(format!("{} is already installed ({version}) — go ahead and use it.", t.name));
            }
            // An unattended run must not queue a software install for later: the
            // work is happening now, and a scheduled review that stops to wait for
            // a Homebrew approval is a review that silently produced nothing.
            // Degrade, report, carry on.
            if ctx.unattended {
                toolchain::record(
                    &mut ctx.coverage.lock().unwrap(),
                    t.check,
                    Status::Skipped { reason: format!("{} is not installed, and nobody was attending this run to approve it", t.name) },
                );
                return Some(format!(
                    "Nobody is attending this run, so {} cannot be installed now — \"{}\" stays recorded as NOT RUN. \
                     Do what you can by hand, `record_coverage` it as degraded if you manage a partial equivalent, \
                     and say in your report what is uncovered.",
                    t.name, t.check
                ));
            }
            let summary = format!("Install {} — {reason}", t.name);
            let payload = serde_json::json!({
                "name": "run_shell",
                "arguments": { "command": t.install, "description": format!("Install {} ({})", t.name, t.check) },
                "reason": if reason.is_empty() { format!("Needed for: {}", t.check) } else { reason.to_string() },
                "category": "shell",
            });
            let decision = match prompt_user(ctx, "permission", &summary, payload, args).await {
                Ok(d) => d.trim().to_string(),
                Err(e) => {
                    toolchain::record(
                        &mut ctx.coverage.lock().unwrap(),
                        t.check,
                        Status::Skipped { reason: format!("{} is not installed and the request reached no one", t.name) },
                    );
                    return Some(format!("{e}\n\n\"{}\" is recorded as NOT RUN.", t.check));
                }
            };
            if decision == "deny" {
                toolchain::record(
                    &mut ctx.coverage.lock().unwrap(),
                    t.check,
                    Status::Skipped { reason: format!("the user declined to install {}", t.name) },
                );
                return Some(format!(
                    "The user declined to install {}. Don't ask again this run. Do the check by hand if you can \
                     ({}) and `record_coverage` it as degraded; otherwise it stays NOT RUN and your report must say so.",
                    t.name, t.fallback
                ));
            }
            crate::core::audit::log(&ctx.data_dir, "approve", &ctx.session_id, t.install, &summary, true);
            let (code, out) = crate::core::coder::run_capture(&ctx.root.to_string_lossy(), t.install, 600, 4000);
            Some(match toolchain::probe(t) {
                Presence::Present { version } => {
                    toolchain::record(
                        &mut ctx.coverage.lock().unwrap(),
                        t.check,
                        Status::Ran { tool: t.name.into(), version: version.clone() },
                    );
                    format!("{} installed ({version}). Go ahead and run the scan.", t.name)
                }
                // Approved, ran, and still not there. Saying "installed" here would
                // be the same lie by a different route.
                Presence::Missing => {
                    toolchain::record(
                        &mut ctx.coverage.lock().unwrap(),
                        t.check,
                        Status::Skipped { reason: format!("installing {} was approved but did not succeed", t.name) },
                    );
                    format!(
                        "The install ran (exit {code}) but {} still isn't on PATH, so \"{}\" stays NOT RUN. \
                         Tell the user what happened and carry on without it.\n\n{out}",
                        t.name, t.check
                    )
                }
            })
        }
        "record_coverage" => {
            let check = args["check"].as_str().unwrap_or("").trim();
            let detail = args["detail"].as_str().unwrap_or("").trim();
            if check.is_empty() || detail.is_empty() {
                return Some("error: `check` and `detail` are both required — a coverage line with no reason is not a coverage line".into());
            }
            let status = match args["status"].as_str().unwrap_or("").trim() {
                "ran" => Status::Ran { tool: detail.to_string(), version: String::new() },
                "degraded" => Status::Degraded { reason: detail.to_string() },
                "skipped" => Status::Skipped { reason: detail.to_string() },
                other => return Some(format!("error: status must be ran | degraded | skipped, not `{other}`")),
            };
            toolchain::record(&mut ctx.coverage.lock().unwrap(), check, status);
            Some(format!("recorded: {check}"))
        }
        "coverage_note" => Some(toolchain::render(&ctx.coverage.lock().unwrap())),
        _ => None,
    }
}
