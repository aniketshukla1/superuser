use super::*;

pub const FILE: &str = "boundaries.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// A refused action does not run.
    #[default]
    Enforce,
    /// A refused action is written down as refused, and runs anyway.
    #[serde(alias = "dry-run", alias = "dry_run")]
    Record,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Enforce => "enforce",
            Mode::Record => "record",
        }
    }
}

/// One rule on the deny list.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct Rule {
    /// What the user calls it — "Never send messages". Optional; the expression is
    /// what is enforced.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// The expression. See `core::rules`.
    pub when: String,
    /// The preset it came from, so the page can show it as that preset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preset: String,
}

// A rule may be written as a bare expression — which is how a policy file
// lists them — or as an object with a name. Both read; the object is what is saved.
impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            Text(String),
            Full {
                #[serde(default)]
                name: String,
                when: String,
                #[serde(default)]
                preset: String,
            },
        }
        Ok(match Either::deserialize(d)? {
            Either::Text(when) => Rule { when, ..Default::default() },
            Either::Full { name, when, preset } => Rule { name, when, preset },
        })
    }
}

pub(super) fn default_allow() -> Vec<String> {
    vec!["true".into()]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boundaries {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub deny: Vec<Rule>,
    #[serde(default = "default_allow")]
    pub allow: Vec<String>,
}

impl Default for Boundaries {
    fn default() -> Self {
        Boundaries { mode: Mode::Enforce, deny: Vec::new(), allow: default_allow() }
    }
}

/// Every field a rule can read, with what it means. The Boundaries page shows this
/// list, and a rule naming anything else is refused before it is saved.
pub const FIELDS: &[(&str, &str)] = &[
    ("intent", "What the action does: read_file, list_files, write_file, run_command, navigate, activate, type, search, send, schedule, remember, delegate, author, read_tool, write_tool, use_computer, show, track, read"),
    ("tool.name", "The tool the assistant called, e.g. run_shell or send_message"),
    ("tool.risk", "read, write_local, exec or external"),
    ("command", "A command about to run, exactly as written"),
    ("file.path", "A file's path, as the assistant gave it"),
    ("file.name", "Just the file's name, e.g. .env"),
    ("file.extension", "Lower-case, without the dot — empty for dotfiles like .env"),
    ("page.url", "An address being fetched or opened, without its query"),
    ("page.host", "Just the host, e.g. example.com"),
    ("target", "What the action touches, in words: a channel, a host, a connected app"),
    ("mcp.server", "The connected app a tool belongs to"),
    ("mcp.tool", "That app's own name for the tool"),
    ("mcp.effect", "read or write — empty for anything that is not a connected app's tool"),
    ("persona.id", "Which assistant is acting"),
    ("initiator.kind", "What started the run: person, approval, schedule, watch, message, subagent or api"),
    ("initiator.id", "The schedule, watch or channel that started it"),
    ("unattended", "true when nobody is watching the run happen"),
    ("tainted", "true once the run has read something from outside this machine — a web page, an email, a connected app"),
    ("chars", "How many characters of text the action carries, e.g. a message's length"),
    ("session.id", "The conversation the run belongs to"),
    ("key", "A key about to be pressed in an assistant's own browser — Enter when typing ends in a submit"),
    ("element.name", "The label of what is being clicked or typed into, as the browser's own snapshot names it"),
    ("element.role", "button, link, textbox, checkbox, …"),
    ("element.ref", "The element's reference in the page snapshot"),
    ("element.type", "An input's type, e.g. password or email"),
    ("bot.id", "The same as persona.id"),
    ("actor.id", "Always \"user\" on this machine"),
];

/// A preset: a rule, and what it costs to have it.
#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub when: &'static str,
    /// What switching this on takes away that the name does not say. It is the part
    /// that makes a boundary a decision rather than a checkbox.
    pub cost: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "no-acting-after-reading-outside",
        name: "Nothing acts after reading outside content",
        when: r#"tainted && intent in ["send", "run_command", "write_tool", "use_computer"]"#,
        cost: "Once a run has read a web page, an email or a connected app, it can't send, run commands or change anything in your apps for the rest of that run — even when what it read was harmless. Start a new turn to act on it.",
    },
    Preset {
        id: "never-send",
        name: "Never send messages",
        when: r#"intent == "send""#,
        cost: "Assistants can still write the message. You send it yourself.",
    },
    Preset {
        id: "no-secret-files",
        name: "Don't touch secret files",
        when: r#"(intent in ["read_file", "write_file", "list_files"] && (file.name == ".env" || startsWith(file.name, ".env.") || file.extension in ["pem", "key", "p12", "pfx", "kdbx"] || contains(file.path, ".ssh/") || contains(file.path, ".aws/") || contains(file.path, ".gnupg/"))) || (intent == "run_command" && (contains(command, ".env") || contains(command, ".ssh/") || contains(command, ".aws/")))"#,
        cost: "Goes by names and folders, so a key saved as notes.txt is not covered, and a script can still read what a command line doesn't mention.",
    },
    Preset {
        id: "no-deleting-commands",
        name: "No commands that delete",
        when: r#"intent == "run_command" && matches(command, r"(^|[;&|(\s])(rm|rmdir|shred|unlink|srm|trash)(\s|$)|git\s+(clean|reset\s+--hard|push\s+(-f|--force))")"#,
        cost: "A filter on the words of a command, not a wall: a script or a program can still delete. It stops the obvious cases, including ones you would have approved.",
    },
    Preset {
        id: "apps-read-only",
        name: "Connected apps are read-only",
        when: r#"intent == "write_tool""#,
        cost: "Assistants can search your issues, pages and messages, but can't file, edit, post or delete anything in those apps.",
    },
    Preset {
        id: "unattended-only-reads",
        name: "Scheduled and background runs only look",
        when: r#"unattended && intent in ["write_file", "run_command", "send", "write_tool", "use_computer", "delegate", "schedule"]"#,
        cost: "Automations can look things up and report back into their conversation, but can't save files, run commands or send anything — including the report files some automations write.",
    },
    Preset {
        id: "never-submit-in-browser",
        name: "Never submit a form in the browser",
        // A form has three doors — the button, Enter
        // in a field, and typing that ends in Enter — and a rule naming one leaves two open.
        when: r#"(intent == "activate" && (contains(element.name, "submit") || contains(element.name, "send") || contains(element.name, "place order") || contains(element.name, "pay") || contains(element.name, "confirm"))) || (tool.name in ["browser_key", "browser_type"] && key == "Enter")"#,
        cost: "Also stops pressing Enter anywhere in the assistant's browser, including in a search box. Buttons are matched by their label, so one that submits while saying \"Continue\" is not covered.",
    },
    Preset {
        id: "no-social-media",
        name: "Stay off social media",
        when: r#"intent == "navigate" && (page.host in ["facebook.com", "x.com", "twitter.com", "instagram.com", "tiktok.com", "reddit.com", "linkedin.com"] || endsWith(page.host, ".facebook.com") || endsWith(page.host, ".x.com") || endsWith(page.host, ".twitter.com") || endsWith(page.host, ".instagram.com") || endsWith(page.host, ".tiktok.com") || endsWith(page.host, ".reddit.com") || endsWith(page.host, ".linkedin.com"))"#,
        cost: "Names these sites only. A link shortener or another site that shows their pages is not covered.",
    },
];

/// Why a boundaries file could not be used, when it could not.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub boundaries: Boundaries,
    /// Rules read and compiled, one per deny rule, in order.
    pub(super) compiled_deny: Vec<Result<Program, rules::RuleError>>,
    pub(super) compiled_allow: Vec<Result<Program, rules::RuleError>>,
    /// Set when the file exists and could not be read. Everything is then refused —
    /// see [`load`].
    pub problem: Option<String>,
}

pub(super) fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}
