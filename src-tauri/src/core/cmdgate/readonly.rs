use super::{program_of, tokenize};

    /// Commands that read and have no writing mode to police.
    const SAFE: &[&str] = &[
        "ls", "cat", "head", "tail", "wc", "nl", "sort", "uniq", "cut", "tr", "rev",
        "grep", "egrep", "fgrep", "rg", "ag", "file", "stat", "du", "df", "pwd",
        "echo", "printf", "which", "type", "whoami", "id", "date", "uname", "hostname",
        "basename", "dirname", "realpath", "readlink", "dirs", "tree", "column",
        "jq", "yq", "diff", "cmp", "md5sum", "shasum", "sha256sum", "cksum",
        "true", "false", "seq", "expr", "env",
    ];

    /// `git` subcommands that only read. Deliberately short: anything that moves a
    /// ref, writes an object, or talks to a remote is absent, and `git config`
    /// stays off because its writing form is a flag away.
    const GIT_READ: &[&str] = &[
        "status", "log", "diff", "show", "blame", "shortlog", "describe",
        "rev-parse", "rev-list", "ls-files", "ls-tree", "cat-file", "grep",
        "reflog", "whatchanged", "count-objects", "verify-pack", "check-ignore",
    ];

    /// `find` reads directories — until one of these turns it into something else.
    /// Checked separately from the general dangerous-flag list because `find` is
    /// too useful in a review to exclude wholesale.
    const FIND_FORBIDDEN: &[&str] = &[
        "-exec", "-execdir", "-delete", "-ok", "-okdir", "-fprintf", "-fprint",
        "-fls", "-fprint0",
    ];

    /// Flags that make an otherwise-reading command write.
    ///
    /// Short, because the list of commands admitted above is short. `tee` is not
    /// here at all — it exists to write, so it never got onto the safe list.
    fn writes_via_flag(program: &str, args: &[String]) -> bool {
        let has = |f: &str| args.iter().any(|a| a == f);
        match program {
            "sort" => has("-o") || has("--output"),
            _ => false,
        }
    }

    /// Does this one command only read local state?
    fn stage_is_read_only(part: &str) -> bool {
        let Some(argv) = tokenize(part) else { return false };
        if argv.is_empty() {
            return false;
        }
        // A path-invoked binary is not the program we think it is: `./ls` is
        // whatever the repo shipped. Only bare names are eligible.
        if argv[0].contains('/') || argv[0].contains('\\') {
            return false;
        }
        let program = program_of(&argv);
        let args = &argv[1..];

        if writes_via_flag(&program, args) {
            return false;
        }
        if SAFE.contains(&program.as_str()) {
            return true;
        }
        match program.as_str() {
            "git" => {
                let Some(sub) = args.iter().find(|a| !a.starts_with('-')) else { return false };
                GIT_READ.contains(&sub.as_str())
            }
            "find" => !args.iter().any(|a| FIND_FORBIDDEN.contains(&a.to_lowercase().as_str())),
            // Deliberately absent: `sed`. `sed -n '10,20p' file` really is just a
            // read, and it is a common one — but `sed` also has `-i`, the `w`
            // command and the `e` command, and telling those apart means parsing a
            // sed script. A classifier that parses one small language to decide
            // whether to skip an approval is a classifier that will eventually
            // parse it wrong in the permissive direction. One click is cheaper.
            _ => false,
        }
    }

    /// Is the whole command read-only?
    ///
    /// Pipelines only. Any other operator, and the answer is no — not because a
    /// `&&` between two reads is dangerous, but because deciding that requires
    /// parsing the shell properly, and a classifier that guesses at shell grammar
    /// is a classifier that will eventually guess wrong in the permissive
    /// direction.
    pub fn is_read_only(command: &str) -> bool {
        let cmd = command.trim();
        if cmd.is_empty() {
            return false;
        }
        // Substitutions and redirects, checked before splitting so `$(…)` inside a
        // stage cannot hide behind a pipe.
        if super::carries_opaque_construct(cmd) {
            return false;
        }
        for op in ["&&", "||", ";", "&", "\n", "\r"] {
            if cmd.contains(op) {
                return false;
            }
        }
        let stages: Vec<&str> = cmd.split('|').map(str::trim).filter(|s| !s.is_empty()).collect();
        if stages.is_empty() {
            return false;
        }
        stages.iter().all(|s| stage_is_read_only(s))
    }
