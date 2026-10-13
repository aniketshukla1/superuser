//! The command gate — what a shell command has to survive before it runs.
//!
//! Two jobs, and they are not the same job.
//!
//! **[`hard_deny`]** is the floor: commands that are catastrophic no matter who
//! asked or what mode we are in. It replaces a substring denylist that was wrong
//! in both directions. `lc.contains("reboot")` blocked `grep reboot syslog`, and
//! `lc.contains("sudo ")` was satisfied by the word appearing inside a quoted
//! string — while `sudo\trm -rf /` (a tab, not a space) walked straight through.
//! Matching argv instead of raw text fixes both: the check now sees the program
//! being run, not the characters that happen to be present.
//!
//! **[`auto_allowable`]** is the ceiling on *silence*: given a list of command
//! prefixes the user has already blessed, may this command run without asking
//! again? That question is much harder than a prefix comparison, because a shell
//! string can smuggle a second command past any rule vouching for the first. The
//! answer here:
//! decompose the command, and refuse eligibility for any part whose real effect
//! cannot be read off its text.
//!
//! Four ways a command carries an effect its prefix never vouched for:
//!
//! 1. **Opaque constructs** — `` ` ``, `$(`, `$VAR`, `>`, `<`, `(`. Substitution
//!    runs code we never parsed; a redirect writes somewhere the rule never
//!    vetted; an expansion's value was set out of view.
//! 2. **Argument executors** — `xargs`, `env`, `sudo`, `ssh`, `docker`, `npx`,
//!    `timeout`. A rule on the outer program can never vouch for the inner one.
//! 3. **Inline code** — `python -c "…"`, `node -e "…"`. The prefix says
//!    "python"; the effect is in a string the prefix never saw.
//! 4. **Execution and deletion flags** — `-exec`, `-delete`, `-ok`. This is why
//!    the list exists at all: `find . -exec rm {} +` needs no separator, so a
//!    `find` prefix rule would auto-run an `rm` without one character of the
//!    command looking unusual.
//!
//! Everything here fails closed. A command we cannot parse is not eligible; an
//! unknown program is not eligible. A false negative costs one approval click, a
//! false positive costs an unreviewed side effect, and that asymmetry decides
//! every judgement call in this file.

/// Commands that only ever read local state.
///
/// This exists because of approval fatigue, which is a security problem and not a
/// comfort one. A security review runs fifteen `ls`, `cat` and `git log` calls
/// before it finds anything, and a user who has clicked Allow fifteen times is a
/// user who will click Allow on the sixteenth without reading it. Removing the
/// cards that carry no decision is what keeps the cards that do carry one worth
/// reading.
///
/// The contract, and every line of it is load-bearing:
///
/// - **Local filesystem reads only.** Network clients — `curl`, `wget`, `ssh`,
///   `nc` — are excluded even for a plain GET. An auto-allowed network command is
///   an exfiltration channel the moment the run is under prompt injection, and
///   "it was only a GET" is no comfort when the data is in the URL.
/// - **No interpreters.** `python`, `sh`, `ruby` and friends read local state
///   right up until the script says otherwise, and the script is not something
///   this function can read.
/// - **Pipelines are allowed** — `git log -p | grep -i password | head -50` is
///   exactly the shape this is for — but every stage must classify on its own.
///   Every other shell operator (`;`, `&&`, `||`, `&`, redirects, substitutions)
///   is rejected outright.
/// - **Fail closed.** An unknown command, an unparseable string, a path-invoked
///   binary, a flag not on the list: all reject. A false negative costs one
///   approval click; a false positive costs an unreviewed side effect.
pub mod readonly;

/// What the user has blessed **for this session** — the memory behind the
/// approval card's "Always allow" buttons.
///
/// Session-scoped and in-memory on purpose. A grant that survived a restart would
/// be a standing rule, and standing rules already exist somewhere with an owner
/// and a revoke path: they live on an automation's task record
/// (`automation::rule_entry`), so deleting the task takes them along. A grant
/// that outlived the conversation it was made in, with nothing listing it and
/// nothing to revoke it from, is how an app ends up permanently permissive
/// without anyone choosing that.
pub mod grants;

#[cfg(test)]
mod tests;

mod tokenize;
mod hard_deny;

pub use tokenize::*;
pub use hard_deny::*;
