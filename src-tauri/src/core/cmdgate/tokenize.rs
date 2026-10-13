
/// Separators that chain several commands into one string, longest first so `&&`
/// is never read as two `&`. Each part is then judged on its own: the older
/// behaviour of rejecting any compound command outright both refused a harmless
/// `git status && git diff` and — because `-exec` needs no separator — still let
/// `find . -exec rm {} +` through.
pub(super) const SEPARATORS: &[&str] = &["&&", "||", ";", "|&", "|", "&", "\n", "\r"];

/// Constructs whose *contents* cannot be evaluated from the text, so a command
/// carrying one is never eligible for prefix auto-run.
pub(super) const OPAQUE_CONSTRUCTS: &[&str] = &["`", "$(", "$", ">", "<", "("];

/// Programs that run *another* program named in their arguments.
pub(super) const ARG_EXECUTORS: &[&str] = &[
    "xargs", "env", "nohup", "nice", "stdbuf", "timeout", "watch", "sudo", "doas",
    "ssh", "docker", "podman", "kubectl", "npx", "pnpx", "bunx", "uvx",
];

/// Interpreters, which are only a problem when carrying inline source.
pub(super) const INTERPRETERS: &[&str] = &[
    "sh", "bash", "zsh", "dash", "ksh", "fish", "powershell", "pwsh", "cmd",
    "python", "python3", "node", "deno", "bun", "ruby", "perl", "php",
];

/// Flags that hand an interpreter its program on the command line.
pub(super) const INLINE_CODE_FLAGS: &[&str] = &["-c", "-e", "--eval", "--command", "-Command", "-EncodedCommand"];

/// Flags that turn a search or list tool into an execution or deletion tool.
pub(super) const DANGEROUS_FLAGS: &[&str] = &["-exec", "-execdir", "-delete", "-ok", "-okdir", "-fprintf"];

/// Split a compound command on its separators. Purely textual — a separator
/// inside quotes splits too. That is deliberate: over-splitting only ever
/// produces more parts to justify, never fewer.
pub fn split_commands(command: &str) -> Vec<String> {
    let mut parts: Vec<String> = vec![command.to_string()];
    for sep in SEPARATORS {
        parts = parts
            .iter()
            .flat_map(|p| p.split(sep).map(|s| s.to_string()).collect::<Vec<_>>())
            .collect();
    }
    parts.into_iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// Split one command part into argv, honouring quotes and backslash escapes.
///
/// `None` when the text cannot be parsed — an unterminated quote, most often.
/// Callers treat that as "not eligible" rather than guessing at the tokens,
/// because a guess here is a guess about what will execute.
pub(super) fn tokenize(part: &str) -> Option<Vec<String>> {
    let mut argv: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut chars = part.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else if c == '\\' && q == '"' {
                    if let Some(n) = chars.next() {
                        cur.push(n);
                    }
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    has_token = true;
                }
                '\\' => {
                    if let Some(n) = chars.next() {
                        cur.push(n);
                        has_token = true;
                    }
                }
                c if c.is_whitespace() => {
                    if has_token {
                        argv.push(std::mem::take(&mut cur));
                        has_token = false;
                    }
                }
                c => {
                    cur.push(c);
                    has_token = true;
                }
            },
        }
    }
    if quote.is_some() {
        return None; // unterminated quote — we do not know what the shell will see
    }
    if has_token {
        argv.push(cur);
    }
    Some(argv)
}

/// The program a part invokes: basename, lowercased, `.exe` stripped. A
/// path-invoked binary (`/usr/bin/sudo`) resolves to the same name as a bare one.
pub(super) fn program_of(argv: &[String]) -> String {
    let raw = argv.first().map(String::as_str).unwrap_or("");
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw).to_lowercase();
    base.strip_suffix(".exe").unwrap_or(&base).to_string()
}

/// Is this argument a flag rather than an operand?
pub(super) fn is_flag(arg: &str) -> bool {
    arg.starts_with('-') && arg != "-"
}

/// Path components of an absolute path, empties dropped: `/usr/local/` → `["usr", "local"]`.
pub(super) fn components(path: &str) -> Vec<&str> {
    path.split('/').filter(|c| !c.is_empty() && *c != ".").collect()
}

/// Would a recursive delete of this operand take out something unrecoverable?
///
/// Denied: `/` itself, the home directory, any single directory directly under
/// `/` (`/usr`, `/etc`, `/Users`), and a user's home root (`/Users/name`,
/// `/home/name`). Left alone: anything deeper, so `rm -rf /tmp/build` and
/// `rm -rf ~/proj/dist` — ordinary, and the reason the old substring rule
/// (`contains("rm -rf /")`, which matched every absolute path) was unusable.
pub(super) fn is_catastrophic_delete_target(operand: &str) -> bool {
    let t = operand.trim_end_matches('/');
    if t.is_empty() || t == "~" || t == "$HOME" || t == "${HOME}" {
        return true; // `rm -rf /` arrives here as the empty string
    }
    if t == "*" || t == "/*" {
        return true;
    }
    if let Some(rest) = t.strip_prefix('~') {
        // `~/` on its own is home; `~/x` is not.
        return rest.trim_matches('/').is_empty();
    }
    if !t.starts_with('/') {
        return false;
    }
    let parts = components(t);
    match parts.len() {
        0 | 1 => true,
        2 => matches!(parts[0], "Users" | "home"),
        _ => false,
    }
}
