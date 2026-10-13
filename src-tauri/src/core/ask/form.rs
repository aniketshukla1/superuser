use super::*;

/// One answer the agent thinks is likely, with the reasoning that distinguishes it
/// from its neighbours.
///
/// The description is not decoration. Four bare labels make the user guess what the
/// difference between them is; a line of explanation under each turns the question
/// into something answerable at a glance.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub description: String,
}

/// One question, with the answers the agent believes are likely.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub options: Vec<Choice>,
    /// Several options can be chosen together.
    pub multi: bool,
}

/// What the agent is putting to the user: up to four questions, asked one at a time.
///
/// **Why options are required.** The first version of this let the agent hand over a
/// form of empty fields, and that is the wrong shape: it moves the thinking from the
/// agent, which has just read the files and knows what the plausible answers are, to
/// the user, who now has to type them out. Making the agent write the options is the
/// point — it cannot list four candidate answers without working out what the answers
/// could be, and quite often that reveals it did not need to ask at all. The card
/// always adds a free-text "Other", so nothing is trapped by a bad list.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Ask {
    pub questions: Vec<Question>,
}

/// One field of a form: a value only the user holds.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Field {
    /// Key the answer comes back under. Stable, machine-ish.
    pub name: String,
    pub label: String,
    /// "text" | "number" | "date". Anything else is read as text — a field the app
    /// cannot render specially is still a field the user can type into.
    pub kind: String,
    pub placeholder: String,
    /// One line under the label, for the thing a label cannot say.
    pub help: String,
    pub required: bool,
}

/// A short form: values the user must supply because nothing can look them up.
///
/// **Why this is not `ask_user`, and must not become it.** The doc on `Ask` above
/// records that a form of empty fields was tried and rejected, because it moves the
/// thinking from the agent — which has just read the files and knows the plausible
/// answers — onto the user. That reasoning stands, and this does not contradict it:
/// it applies to JUDGMENT, where options are the work.
///
/// This is for the other kind of gap. A target margin, a start date, an account id,
/// the address of a staging box — facts only the user holds, where there is nothing
/// to enumerate and offering "2 to 4 likely answers" would be inventing them. The
/// line between the two tools is exactly that: if you can list the plausible answers,
/// you owe the user options (`ask_user`); if there is nothing to list, ask for the
/// value.
///
/// The rule `ask_user` carries about findable things applies here twice over: a file
/// path, a filename, a column name are all things to go and look up, and a form is a
/// tempting place to ask for a pile of them at once.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Form {
    pub title: String,
    /// Why you are being asked, in one line.
    pub intro: String,
    pub fields: Vec<Field>,
}

/// Most fields in one form.
///
/// Six. Past that it is an intake form, and an agent reaching for ten has usually not
/// worked out which two it actually cannot proceed without.
pub(super) const MAX_FIELDS: usize = 6;
