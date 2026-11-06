use super::*;

/// How far a chain of asks may run from the person: A asks B, B asks C, and there it
/// stops. Deeper chains are where the work stops being traceable to the request.
pub const MAX_DEPTH: u8 = 2;

/// How many assistants one turn may ask. Three is enough to split a question; more is a
/// run that has stopped answering and started managing.
pub const MAX_PER_TURN: usize = 3;

/// How long the asked assistant has to answer. The asking run is holding a person's turn
/// open, so this is short by design; a piece of work that needs longer is work to hand to
/// an automation rather than to a coworker.
pub const DEADLINE_SECS: u64 = 300;

/// How much of the answer is relayed back before it is put in a file instead.
pub const RELAY_LIMIT: usize = 12_000;

/// An assistant this one may ask.
#[derive(Debug, Clone, PartialEq)]
pub struct Coworker {
    pub id: String,
    pub name: String,
    pub tagline: String,
}

/// What one assistant asks another.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    pub task: String,
    pub constraints: String,
    pub expecting: String,
}

impl Envelope {
    /// The three fields, or the sentence to hand back. A handoff with no task is the one
    /// that cannot be repaired at the other end.
    pub fn parse(args: &Value) -> Result<Self, String> {
        let get = |k: &str| args[k].as_str().unwrap_or("").trim().to_string();
        let task = get("task");
        if task.is_empty() {
            return Err("Nothing was sent: say what you are asking them to do.".into());
        }
        Ok(Envelope { task, constraints: get("constraints"), expecting: get("expecting") })
    }

    /// What the asked assistant reads. It is told who asked, what for, and that the person
    /// is not in the room to be asked anything.
    pub fn brief(&self, from_name: &str) -> String {
        let mut out = format!(
            "{from_name} needs your help with this, for the person it works for.\n\nTask: {}\n",
            self.task
        );
        if !self.constraints.is_empty() {
            out.push_str(&format!("Constraints: {}\n", self.constraints));
        }
        if !self.expecting.is_empty() {
            out.push_str(&format!("What a good answer looks like: {}\n", self.expecting));
        }
        out.push_str(
            "\nAnswer as yourself and write it for the person, not for the assistant that asked. \
             You can't put a question to the person from here, so if something is missing, say what \
             it is in your answer. If you can't do this, say so plainly and say why — don't guess at it.",
        );
        out
    }

    /// The one line the person sees in the asked assistant's own conversation. The
    /// constraints and the note on the answer's shape are left out: they are written for a
    /// model, and a transcript full of them would no longer read like a conversation.
    pub fn shown(&self, from_name: &str) -> String {
        format!("{from_name} asked: {}", self.task)
    }

    /// What makes two asks the same ask. Hashed rather than kept, because the task is the
    /// asking model's words about the work and this only has to answer "again?".
    pub fn key(&self, to: &str) -> String {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&(to, &self.task, &self.constraints, &self.expecting), &mut h);
        format!("{:x}", std::hash::Hasher::finish(&h))
    }
}

/// How far from the person this run already is, read off its session id.
///
/// The id carries it because the trail already carries the id: a row from `ask2-91f0` says
/// what it is without a second field to keep in step, and a run cannot relabel itself by
/// claiming a different depth — it would have to be in a different conversation.
pub fn depth_of(session_id: &str) -> u8 {
    let Some(rest) = session_id.strip_prefix("ask") else { return 0 };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || !rest[digits.len()..].starts_with('-') {
        return 0;
    }
    digits.parse().unwrap_or(0)
}

/// The session id for a hop from a run at this depth.
pub fn session_id(depth: u8) -> String {
    format!("ask{}-{}", depth + 1, crate::core::inbox::short_id())
}

/// Who this assistant may ask: what the person granted, minus itself and anything
/// switched off. An assistant that may ask nobody is offered no tool at all.
pub fn may_ask(reg: &PersonaRegistry, from: &str) -> Vec<Coworker> {
    reg.asks(from)
        .into_iter()
        .filter(|id| id != from)
        .filter_map(|id| {
            let m = reg.manifest(&id)?;
            reg.is_enabled(&id).then(|| Coworker { id: m.id.clone(), name: m.name.clone(), tagline: m.tagline.clone() })
        })
        .collect()
}

/// The roster, for the prompt. Ids as well as names: a model that has both can address
/// one of two assistants called "Analyst" without a round trip.
pub fn roster_block(roster: &[Coworker]) -> String {
    if roster.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\n\n## Assistants you can ask\n\nEach answers as itself, with its own tools and its own access. \
         Ask one when the work needs a role you don't have — not to check your own work, and not for \
         something the person is better placed to answer.\n\n",
    );
    for c in roster {
        out.push_str(&format!("- **{}** (`{}`){}\n", c.name, c.id, if c.tagline.is_empty() { String::new() } else { format!(" — {}", c.tagline) }));
    }
    out
}

/// Which assistant a name or id means, or the sentence to hand back.
///
/// Id first, then name, because a name is not unique: duplicating an assistant makes a
/// second one with the same name, and picking one of them at random is worse than saying
/// so. An assistant that exists but was not granted is refused in its own words — but one
/// that does not exist and one that is switched off are refused in the SAME words, so the
/// refusal cannot be used to find out what else is installed.
pub fn resolve(reg: &PersonaRegistry, from: &str, target: &str) -> Result<Coworker, String> {
    let wanted = target.trim();
    if wanted.is_empty() {
        return Err("Nothing was sent: name the assistant you are asking.".into());
    }
    let unknown = || format!("There's no assistant called “{}” that you can ask.", wanted.chars().take(60).collect::<String>());
    let granted = may_ask(reg, from);
    // Self first: "you can't ask yourself" is the truer answer than "you weren't granted
    // yourself", whichever way the person set the grants up.
    let is_self = |id: &str| id == from;
    let by_id = reg.manifest(wanted).map(|m| m.id.clone()).filter(|id| reg.is_enabled(id));
    if let Some(id) = by_id {
        if is_self(&id) {
            return Err(SELF_REFUSAL.into());
        }
        return granted
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| ungranted(reg.manifest(&id).map(|m| m.name.as_str()).unwrap_or(wanted)));
    }
    let same_name = |name: &str| name.eq_ignore_ascii_case(wanted);
    if reg.manifest(from).is_some_and(|m| same_name(&m.name)) {
        return Err(SELF_REFUSAL.into());
    }
    let named: Vec<Coworker> = reg
        .list_all()
        .into_iter()
        .filter(|p| p.enabled && same_name(&p.name))
        .map(|p| Coworker { id: p.id, name: p.name, tagline: p.tagline })
        .collect();
    match named.len() {
        0 => Err(unknown()),
        1 => {
            let found = named.into_iter().next().expect("one");
            granted.into_iter().find(|c| c.id == found.id).ok_or_else(|| ungranted(&found.name))
        }
        _ => Err(format!(
            "More than one assistant is called “{}”. Ask for one by its id: {}.",
            wanted,
            named.iter().map(|c| format!("`{}`", c.id)).collect::<Vec<_>>().join(", ")
        )),
    }
}

pub(super) const SELF_REFUSAL: &str = "You can't hand work to yourself. Do it, or ask the person.";

pub(super) fn ungranted(name: &str) -> String {
    format!("You haven't been given {name} to ask. The person decides that, on your page, under who you may ask.")
}
