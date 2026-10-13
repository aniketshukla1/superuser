use super::*;

/// Catastrophic commands, refused in every mode and for every persona.
///
/// Returns the reason to show, or `None` to let the command through to the
/// normal permission flow. This is a floor, not the whole gate: passing here
/// means "not unrecoverable", never "safe".
pub fn hard_deny(command: &str) -> Option<String> {
    // Two patterns argv can never see, because they are syntax rather than
    // arguments: the fork bomb, and a redirect onto a raw disk device.
    let squashed: String = command.chars().filter(|c| !c.is_whitespace()).collect();
    if squashed.contains(":(){") {
        return Some("that is a fork bomb — it would hang the machine".into());
    }
    for dev in ["/dev/sd", "/dev/disk", "/dev/rdisk", "/dev/nvme", "/dev/hd"] {
        if squashed.contains(&format!(">{dev}")) {
            return Some(format!("that writes straight onto the disk device {dev}…, which destroys the filesystem"));
        }
    }

    for part in split_commands(command) {
        let Some(argv) = tokenize(&part) else { continue };
        if argv.is_empty() {
            continue;
        }
        let program = program_of(&argv);
        let args: Vec<&str> = argv[1..].iter().map(String::as_str).collect();

        if matches!(program.as_str(), "sudo" | "doas" | "su") {
            return Some(format!("`{program}` runs as another user — do it yourself in a terminal if you mean it"));
        }
        if program.starts_with("mkfs") {
            return Some("`mkfs` formats a filesystem, erasing everything on it".into());
        }
        if matches!(program.as_str(), "shutdown" | "reboot" | "halt" | "poweroff") {
            return Some(format!("`{program}` would take the machine down"));
        }
        if program == "dd" {
            if let Some(out) = args.iter().find(|a| a.starts_with("of=/dev/")) {
                return Some(format!("`dd {out}` writes to a device, which destroys what is on it"));
            }
        }
        if program == "rm" {
            if args.contains(&"--no-preserve-root") {
                return Some("`rm --no-preserve-root` exists only to delete the root filesystem".into());
            }
            let recursive = args.iter().any(|a| {
                *a == "-r" || *a == "-R" || *a == "--recursive"
                    || (is_flag(a) && !a.starts_with("--") && a.contains('r'))
            });
            if recursive {
                if let Some(bad) = args.iter().find(|a| !is_flag(a) && is_catastrophic_delete_target(a)) {
                    return Some(format!("a recursive delete of `{bad}` is not recoverable"));
                }
            }
        }
    }
    None
}

/// Does the command carry a construct whose contents cannot be read off the text?
pub fn carries_opaque_construct(command: &str) -> bool {
    OPAQUE_CONSTRUCTS.iter().any(|c| command.contains(c))
}

/// False when a parsed command can never be vouched for by a prefix rule,
/// because it runs code the rule never saw.
pub fn prefix_eligible(argv: &[String]) -> bool {
    if argv.is_empty() {
        return false;
    }
    let program = program_of(argv);
    if ARG_EXECUTORS.contains(&program.as_str()) {
        return false;
    }
    let args = &argv[1..];
    if INTERPRETERS.contains(&program.as_str())
        && args.iter().any(|a| INLINE_CODE_FLAGS.contains(&a.as_str()))
    {
        return false;
    }
    if args.iter().any(|a| DANGEROUS_FLAGS.contains(&a.to_lowercase().as_str())) {
        return false;
    }
    true
}

/// May this command run without asking, given the prefixes already blessed?
///
/// Every part must independently clear [`prefix_eligible`] *and* match a rule.
/// One unjustifiable part disqualifies the whole string — the user approved a
/// command, not a line of shell that happens to start with one.
///
/// Rules are command prefixes as text (`"git status"`, `"cargo test"`), matched
/// token-wise so `"git s"` does not vouch for `git status` and `"git status"`
/// does not vouch for `git statusfoo`.
pub fn auto_allowable(command: &str, allowlist: &[String]) -> bool {
    if allowlist.is_empty() || command.trim().is_empty() {
        return false;
    }
    if carries_opaque_construct(command) {
        return false;
    }
    if hard_deny(command).is_some() {
        return false;
    }
    let rules: Vec<Vec<String>> = allowlist.iter().filter_map(|r| tokenize(r)).filter(|r| !r.is_empty()).collect();
    if rules.is_empty() {
        return false;
    }
    let parts = split_commands(command);
    if parts.is_empty() {
        return false;
    }
    parts.iter().all(|part| {
        let Some(argv) = tokenize(part) else { return false };
        prefix_eligible(&argv) && rules.iter().any(|rule| argv.len() >= rule.len() && argv[..rule.len()] == rule[..])
    })
}
