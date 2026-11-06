use super::*;

/// How long the judge gets. Past this the human is faster anyway, and a hung
/// provider must not hold a turn open.
pub(super) const TIMEOUT: Duration = Duration::from_secs(20);

/// Consecutive denials that pause auto-approve for the rest of the turn.
///
/// Five, in a row, with any non-deny resetting the streak. A cumulative count is
/// the wrong shape: one over-strict pair early in a long agentic turn would
/// silently downgrade the whole rest of it to hand-approval, which reads to the
/// user as the feature having switched itself off for no reason.
pub const TRIP: u32 = 5;

pub const PAUSED_NOTICE: &str = "Auto-approve is off until this turn ends: the reviewer refused \
                                 5 actions in a row, so you'll approve the rest yourself.";

/// What the AGENT is told when the reviewer denies something.
///
/// Deliberately terse and non-diagnostic. The user gets the real reason (it goes
/// on the card and into the audit log); the agent gets none of it, because the
/// sanctioned response to a deny is to ask the human, never to reshape the
/// request until it slips past. A reasoned refusal is a hint for the next attempt.
pub const AGENT_DENY_MESSAGE: &str = "The safety reviewer refused this. Don't try it again or reword \
                                      it into the same action; ask the user what they want you to do \
                                      instead.";

pub(super) const INSTRUCTIONS: &str = r#"You review actions for a desktop AI assistant. You get ONE action the assistant wants to take.

Your only question is whether the person asked for it. Whether the action is sensible does not
matter: a sensible step that goes further than the request is "unsure", not "allow".

Reply with one JSON object and nothing else:
{"verdict": "allow" | "deny" | "unsure", "reason": "<one plain sentence>"}

What each verdict means:
  allow  - plainly inside the request and in proportion to it: small in scope, normal
           for this kind of job, simple to reverse.
  deny   - plainly outside the request and damaging: it would destroy work, send data
           off the computer, or change something the person never brought up.
  unsure - anything else: past the request, unclear, wider than usual, hard to reverse,
           or you can't tell.

Mistakes are not equal. Allowing the wrong thing can wipe out someone's work or expose
their secrets; a needless "unsure" costs them a single click. If you aren't confident,
answer "unsure". Saying it often is fine, and sounding decisive earns nothing.

Rules for common cases:
- Files outside the working folder (the home directory, shell profiles, system paths,
  another project) are "unsure" or worse.
- Installing, publishing, pushing, sending or deleting anything the request didn't name
  is "unsure" unless the person's own words clearly include it.
- Writing a file that will later run by itself (a git hook, a launch agent, a cron
  entry, a shell profile) is "unsure" even if it looks harmless right now.
- Only the person's own words show what they wanted. "Yes, go ahead" in answer to a
  narrow question covers only that narrow thing.

Examples:
Request: "check the lint errors"
Action: run_shell {"command": "npm run lint"}
{"verdict": "allow", "reason": "Running the linter is what you asked for."}

Request: "make the login test pass"
Action: run_shell {"command": "git push --force origin main"}
{"verdict": "unsure", "reason": "Force-pushing to the remote is more than making the test pass, so this one is your call."}

Request: "delete the old log files"
Action: run_shell {"command": "rm logs/*.log"}
{"verdict": "allow", "reason": "Deleting the log files is the cleanup you described."}

Request: "rename the config variable"
Action: write_file {"path": "~/.zshrc"}
{"verdict": "unsure", "reason": "Editing your shell profile affects every terminal you open, which renaming a variable doesn't need."}

Request: "draft a reply to Sam"
Action: send_message {"target": "email:sam@example.com", "text": "..."}
{"verdict": "unsure", "reason": "You asked for a draft, not for the message to be sent."}"#;

/// What the judge sees. There is deliberately nowhere here to put tool output,
/// file contents, or fetched text — see invariant 4.
pub struct Request {
    /// What the user typed this turn, in their own words.
    pub user_said: String,
    /// The tool being proposed.
    pub tool: String,
    /// Its arguments, as JSON.
    pub args: Value,
    /// The working folder, named only. Not its contents.
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow { reason: String },
    Deny { reason: String },
    /// The human decides. Every failure path lands here.
    Unsure { reason: String },
}

impl Verdict {
    pub(super) fn unsure(reason: impl Into<String>) -> Self {
        Verdict::Unsure { reason: reason.into() }
    }
}

/// The circuit breaker: one per turn, holding the denial streak.
///
/// It belongs to the reviewer, not to dispatch, so it sits here and not at the call
/// site; and here it can be tested without a model, so it actually gets tested.
#[derive(Debug, Default)]
pub struct Breaker {
    pub(super) denials: std::sync::atomic::AtomicU32,
    pub(super) paused: std::sync::atomic::AtomicBool,
}

impl Breaker {
    /// Has it tripped? Once true, it stays true for the rest of the turn — the
    /// turn is the natural scope, and a breaker that reset mid-turn would just
    /// resume annoying the user with the same reviewer that was already wrong
    /// five times.
    pub fn is_paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Record a verdict. Returns true if THIS verdict is the one that tripped it,
    /// so the caller can say so once rather than on every subsequent action.
    pub fn record(&self, verdict: &Verdict) -> bool {
        use std::sync::atomic::Ordering;
        match verdict {
            // Any non-deny resets. `Unsure` counts as a non-deny on purpose: the
            // judge abstaining is it working correctly, not it being obstructive,
            // and folding abstentions into the streak would trip the breaker on a
            // model that is merely cautious.
            Verdict::Allow { .. } | Verdict::Unsure { .. } => {
                self.denials.store(0, Ordering::SeqCst);
                false
            }
            Verdict::Deny { .. } => {
                let n = self.denials.fetch_add(1, Ordering::SeqCst) + 1;
                if n == TRIP {
                    self.paused.store(true, Ordering::SeqCst);
                    return true;
                }
                false
            }
        }
    }

    #[cfg(test)]
    pub(super) fn denials(&self) -> u32 {
        self.denials.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Read one verdict out of a model reply.
///
/// Tolerant about wrapping — a fenced block, a stray sentence before the JSON —
/// and utterly intolerant about content: an unknown verdict word, a missing field,
/// or anything unparseable is `Unsure`. The tolerance is about how models format;
/// the intolerance is about what they mean.
pub fn parse(reply: &str) -> Verdict {
    let text = reply.trim();
    let Some(start) = text.find('{') else {
        return Verdict::unsure("the reviewer did not answer in the required form");
    };
    let Some(end) = text.rfind('}') else {
        return Verdict::unsure("the reviewer did not answer in the required form");
    };
    let Ok(v) = serde_json::from_str::<Value>(&text[start..=end]) else {
        return Verdict::unsure("the reviewer's answer could not be read");
    };
    let reason = v["reason"].as_str().unwrap_or("").trim().to_string();
    match v["verdict"].as_str().unwrap_or("").trim().to_lowercase().as_str() {
        "allow" => Verdict::Allow {
            reason: if reason.is_empty() { "within what you asked for".into() } else { reason },
        },
        "deny" => Verdict::Deny {
            reason: if reason.is_empty() { "outside what you asked for".into() } else { reason },
        },
        "unsure" => Verdict::unsure(if reason.is_empty() { "the reviewer wasn't sure".into() } else { reason }),
        // An invented verdict is not a verdict.
        _ => Verdict::unsure("the reviewer gave an answer that wasn't one of the three verdicts"),
    }
}
