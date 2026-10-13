//! What the user is looking at.
//!
//! An assistant sitting beside someone can see their screen; this one cannot, so
//! "change that to fifteen" has no referent and it has to ask which file, or guess.
//! The panel already knows which documents are open and which one is in front —
//! this carries that across so the obvious sentence works.
//!
//! Deliberately ephemeral. It is a description of a window, not a fact about the
//! session: a stale one would have the agent confidently editing a file the user
//! closed yesterday, which is worse than not knowing.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenDocs {
    /// Workspace-relative paths, in tab order.
    #[serde(default)]
    pub open: Vec<String>,
    /// The one in front, if any.
    #[serde(default)]
    pub active: Option<String>,
}

/// Per-session, in memory only.
fn store() -> &'static Mutex<HashMap<String, OpenDocs>> {
    static S: std::sync::OnceLock<Mutex<HashMap<String, OpenDocs>>> = std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}

pub fn set(session_id: &str, docs: OpenDocs) {
    let mut m = store().lock().unwrap();
    if docs.open.is_empty() && docs.active.is_none() {
        m.remove(session_id);
    } else {
        m.insert(session_id.to_string(), docs);
    }
}

pub fn get(session_id: &str) -> OpenDocs {
    store().lock().unwrap().get(session_id).cloned().unwrap_or_default()
}

pub fn clear(session_id: &str) {
    store().lock().unwrap().remove(session_id);
}

/// The sentence added to a turn's system prompt.
///
/// Phrased as what the user can SEE rather than as an instruction, because it is
/// context, not a task: told "edit this file", a model will go and edit it whether
/// or not that was asked. The empty case returns nothing at all — a paragraph
/// explaining that no documents are open costs tokens on every turn to say nothing.
pub fn prompt_block(docs: &OpenDocs) -> String {
    if docs.open.is_empty() && docs.active.is_none() {
        return String::new();
    }
    let mut s = String::from("\n\nOpen in the user's panel right now");
    match (&docs.active, docs.open.len()) {
        (Some(a), 1) => s.push_str(&format!(": {a}. \"this file\" or \"that spreadsheet\" means this one.")),
        (Some(a), _) => {
            let others: Vec<&String> = docs.open.iter().filter(|o| *o != a).collect();
            s.push_str(&format!(
                ", in front: {a} — \"this file\" means that one. Also open: {}.",
                others.iter().map(|o| o.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        (None, _) => s.push_str(&format!(": {}.", docs.open.join(", "))),
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs(open: &[&str], active: Option<&str>) -> OpenDocs {
        OpenDocs {
            open: open.iter().map(|s| s.to_string()).collect(),
            active: active.map(String::from),
        }
    }

    #[test]
    fn nothing_open_costs_nothing() {
        // This rides on every turn; a paragraph saying "no documents" would be paid
        // for forever to convey nothing.
        assert_eq!(prompt_block(&OpenDocs::default()), "");
    }

    #[test]
    fn one_open_document_becomes_the_referent_for_this_file() {
        let b = prompt_block(&docs(&["budget.xlsx"], Some("budget.xlsx")));
        assert!(b.contains("budget.xlsx"));
        assert!(b.contains("this file"), "the phrase the user will actually say is not bound: {b}");
    }

    #[test]
    fn with_several_open_the_front_one_wins_and_the_rest_are_still_named() {
        let b = prompt_block(&docs(&["a.xlsx", "b.docx", "c.pdf"], Some("b.docx")));
        assert!(b.contains("in front: b.docx"));
        assert!(b.contains("a.xlsx") && b.contains("c.pdf"), "the others should still be reachable: {b}");
        // The active one must not be listed twice.
        assert_eq!(b.matches("b.docx").count(), 1);
    }

    #[test]
    fn documents_open_with_none_in_front_are_still_offered() {
        let b = prompt_block(&docs(&["a.xlsx", "b.pdf"], None));
        assert!(b.contains("a.xlsx") && b.contains("b.pdf"));
        assert!(!b.contains("in front"));
    }

    #[test]
    fn it_describes_rather_than_instructs() {
        // Told to edit something, a model will edit it. This is context: what the
        // user can see, not a task they asked for.
        let b = prompt_block(&docs(&["budget.xlsx"], Some("budget.xlsx")));
        let lower = b.to_lowercase();
        for imperative in ["edit ", "update ", "you must", "modify "] {
            assert!(!lower.contains(imperative), "reads as an instruction ({imperative}): {b}");
        }
    }

    #[test]
    fn a_session_only_sees_its_own_open_documents() {
        set("s-one", docs(&["one.xlsx"], Some("one.xlsx")));
        set("s-two", docs(&["two.xlsx"], Some("two.xlsx")));
        assert_eq!(get("s-one").active.as_deref(), Some("one.xlsx"));
        assert_eq!(get("s-two").active.as_deref(), Some("two.xlsx"));
        clear("s-one");
        clear("s-two");
    }

    #[test]
    fn closing_everything_forgets_the_session_rather_than_remembering_an_empty_panel() {
        // A stale entry would have the agent editing a file the user closed.
        set("s-three", docs(&["x.xlsx"], Some("x.xlsx")));
        set("s-three", OpenDocs::default());
        assert_eq!(get("s-three"), OpenDocs::default());
    }

    #[test]
    fn an_unknown_session_has_nothing_open() {
        assert_eq!(get("never-seen"), OpenDocs::default());
    }
}
