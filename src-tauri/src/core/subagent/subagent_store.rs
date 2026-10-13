use super::*;

/// Tools no subagent may use, whatever its type says.
///
/// `send_message` and the scheduler act on the user's channels in the *main*
/// agent's name, which is outside the scope of one delegated task. `computer_use`
/// drives the whole desktop, which is not something to hand to a background worker
/// nobody is watching. `spawn_subagent` would let a general-purpose type recurse.
pub const BLOCKED: &[&str] = &["send_message", "computer_use", "spawn_subagent", "connector_sync", "ask_user"];

/// A subagent gets half the parent's step budget: its task is already bounded, so
/// it does not need the allowance of a whole conversation. When it runs out it is
/// asked to summarize what it managed.
pub fn step_budget(parent: usize) -> usize {
    (parent / 2).max(4)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentType {
    pub name: String,
    /// What the parent selects on — this should say *when* to use the type, not
    /// what it is.
    pub description: String,
    /// Allowed tool names. `None` inherits the parent's tools (minus `BLOCKED`).
    pub tools: Option<Vec<String>>,
    /// The type's system prompt: how to work and what to return.
    pub instructions: String,
    pub builtin: bool,
}

pub(super) const GENERAL_PURPOSE: &str = "You are a subagent handling one self-contained task for another agent.

You cannot see the conversation this came from and you cannot ask questions — everything
you need is in the task. If something essential is genuinely missing, say so plainly in
your answer rather than guessing at it.

Work through the task with the tools you have, then return ONLY the conclusion: what you
found or did, the specific facts that matter (names, paths, numbers, URLs), and anything
you could not determine. Do not narrate your steps — the agent that called you sees only
this answer.";

pub(super) const EXPLORE: &str = "You are a read-only research subagent. You investigate and report; you never change anything.

You cannot see the conversation this came from and you cannot ask questions.

How to work:
1. Search broadly first, then follow the two or three most promising leads.
2. Prefer primary sources — the file itself, the official docs, the original announcement —
   over anything describing them.
3. Cross-check any number, date or version against a second source where you can.

Return a short report, in this order:
- Answer: two or three sentences that settle the question
- Findings: bullets, each with where it came from (a file path or a URL)
- Unconfirmed: anything you could not verify

Write \"not found\" where you came up empty. Never fill a gap with a guess.";

/// The read-only toolset. Named explicitly rather than derived from `RiskClass`
/// so that adding a new read-class tool elsewhere cannot silently widen it.
pub(super) const VERIFY: &str = "You are a verification subagent. Someone else built this; you check whether it actually works.

You cannot see the conversation this came from and you cannot ask questions. The task names the
board item and its acceptance criteria — those criteria are the whole test. Not whether the code
looks reasonable, not whether you would have done it that way: whether what was asked for is true.

How to work:
1. Read the acceptance criteria first, before the diff. Deciding what \"done\" means after seeing
   the work is how a review becomes a rubber stamp.
2. Check it yourself. Run the tests, read the output, open the file. \"The diff looks correct\" is
   not evidence and will be refused.
3. Look for what the change breaks, not only what it fixes.

Record your verdict with `board_review`, with evidence: what you ran, what it printed, what you
read. Then return a two-line summary. If it fails, say precisely which criterion and how you know.

You may not mark anything done. Passing your review is what lets someone else do that.";

/// The verifier's toolset: enough to actually check, and `board_review` to record
/// the verdict. No writing tools at all — a reviewer that can edit the work is a
/// reviewer that can make it pass.
pub(super) const VERIFY_TOOLS: &[&str] = &[
    "read_file", "list_dir", "grep", "git_status", "git_log", "git_diff",
    "run_shell", "board_list", "board_review", "board_note",
];

pub(super) const EXPLORE_TOOLS: &[&str] = &[
    "read_file", "list_dir", "grep", "git_status", "git_log", "git_diff",
    "web_search", "web_fetch", "recall_memory", "list_connectors", "connector_search", "load_skill",
];

pub struct SubagentStore {
    pub(super) types: BTreeMap<String, SubagentType>,
}

impl SubagentStore {
    /// Built-ins first, so a user file of the same name replaces one — the same
    /// precedence skills use.
    pub fn load(data_dir: &Path) -> Self {
        let mut types = BTreeMap::new();
        for t in [
            SubagentType {
                name: "general-purpose".into(),
                description: "Multi-step work involving both investigation and action: searching, reading, running commands, writing files.".into(),
                tools: None,
                instructions: GENERAL_PURPOSE.into(),
                builtin: true,
            },
            SubagentType {
                name: "verify".into(),
                description: "Check work someone else did against its acceptance criteria, and record the verdict. Cannot edit the work it is judging.".into(),
                tools: Some(VERIFY_TOOLS.iter().map(|s| s.to_string()).collect()),
                instructions: VERIFY.into(),
                builtin: true,
            },
            SubagentType {
                name: "explore".into(),
                description: "Read-only investigation: finding files, searching code or documents, gathering facts from the web. Cannot change anything.".into(),
                tools: Some(EXPLORE_TOOLS.iter().map(|s| s.to_string()).collect()),
                instructions: EXPLORE.into(),
                builtin: true,
            },
        ] {
            types.insert(t.name.clone(), t);
        }

        if let Ok(rd) = fs::read_dir(data_dir.join("subagents")) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.extension().map(|x| x != "md").unwrap_or(true) {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else { continue };
                let fallback = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                // A malformed type is skipped rather than breaking the agent.
                if let Ok(t) = parse_type(&text, fallback) {
                    types.insert(t.name.clone(), t);
                }
            }
        }
        Self { types }
    }

    pub fn get(&self, name: &str) -> Option<&SubagentType> {
        self.types.get(name)
    }

    pub fn names(&self) -> Vec<String> {
        self.types.keys().cloned().collect()
    }

    /// The menu the parent chooses from, for the `spawn_subagent` tool description.
    pub fn catalog_text(&self) -> String {
        self.catalog_text_for(&[])
    }

    /// The menu an assistant is shown, narrowed to the roster it staffs.
    ///
    /// An empty roster means the whole catalogue — what every assistant saw before
    /// rosters existed, and still right for a generalist that delegates
    /// opportunistically. A named roster is for a lead: it should be handing work
    /// to types it was designed around, not to whatever happens to be installed on
    /// this machine.
    ///
    /// A roster naming a type that is not installed narrows to nothing rather than
    /// silently falling back to everything — quietly widening a deliberate list is
    /// how a lead ends up delegating to a worker nobody vetted. `missing_from`
    /// reports the gap so a surface can say so.
    pub fn catalog_text_for(&self, roster: &[String]) -> String {
        self.types
            .values()
            .filter(|t| roster.is_empty() || roster.iter().any(|r| r == &t.name))
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Types a roster names that are not installed.
    pub fn missing_from(&self, roster: &[String]) -> Vec<String> {
        roster.iter().filter(|r| !self.types.contains_key(*r)).cloned().collect()
    }
}
