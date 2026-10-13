use super::*;

impl Ask {
    /// Read an `ask_user` call. Accepts the list form and the one-question shorthand,
    /// because a model asking a single thing reaches for the flat shape every time.
    pub fn parse(args: &Value) -> Result<Self, String> {
        let raw: Vec<Value> = match args["questions"].as_array() {
            Some(a) => a.clone(),
            None => vec![args.clone()],
        };
        let mut questions = Vec::new();
        for q in raw.iter().take(MAX_QUESTIONS) {
            match Question::parse(q) {
                Some(Ok(q)) => questions.push(q),
                // One malformed question fails the call rather than being dropped: a
                // silently shortened ask is one the model believes it made in full.
                Some(Err(e)) => return Err(e),
                None => continue,
            }
        }
        if questions.is_empty() {
            return Err("error: `questions` must contain at least one question with a `question` and its likely `options`.".into());
        }
        Ok(Ask { questions })
    }

    /// One line naming what is being asked, for the step row and the Inbox.
    pub fn summary(&self) -> String {
        match self.questions.len() {
            1 => self.questions[0].question.clone(),
            n => format!("{} ({n} questions)", self.questions[0].question),
        }
    }

    pub fn payload(&self) -> Value {
        json!({ "questions": self.questions })
    }

    /// Turn the card's reply into the answer the model reads.
    ///
    /// A skipped question is reported as skipped, in as many words. Leaving it out
    /// would hand back a shorter list than was asked, which reads as "that question
    /// didn't happen" — and the gap is exactly where a model fills in its own guess.
    pub fn read_answer(&self, raw: &str) -> String {
        let parsed: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
        let answers = parsed["answers"].as_array().cloned().unwrap_or_default();
        if answers.is_empty() {
            // Not a card reply — someone answered in prose, from the Inbox or an
            // older client. Their words are still their answer.
            return format!("The user answered: {}", raw.trim());
        }
        let mut lines = Vec::new();
        let mut skipped = 0;
        for (i, q) in self.questions.iter().enumerate() {
            let a = answers.get(i).and_then(|a| a.as_str()).map(str::trim).filter(|a| !a.is_empty());
            match a {
                Some(a) => lines.push(format!("{}. {} → {a}", i + 1, q.question)),
                None => {
                    skipped += 1;
                    lines.push(format!("{}. {} → (skipped)", i + 1, q.question));
                }
            }
        }
        format!(
            "The user answered:\n{}{}",
            lines.join("\n"),
            if skipped > 0 {
                "\n\nThey skipped the ones marked (skipped). Do not fill those in yourself — either work without them, or say what you still need."
            } else {
                ""
            }
        )
    }
}

pub(super) fn pending() -> &'static Mutex<HashMap<String, oneshot::Sender<String>>> {
    static P: OnceLock<Mutex<HashMap<String, oneshot::Sender<String>>>> = OnceLock::new();
    P.get_or_init(Default::default)
}

/// Register a question for `session_id` and hand back the receiver to await.
///
/// One question per session at a time, on purpose. A model that asks twice in one
/// step has not asked two questions, it has changed its mind — so the newer one
/// replaces the older, and the older's receiver ends with the sender dropped, which
/// the caller reports as superseded rather than as an answer.
pub fn park(session_id: &str) -> oneshot::Receiver<String> {
    let (tx, rx) = oneshot::channel();
    if let Ok(mut p) = pending().lock() {
        p.insert(session_id.to_string(), tx);
    }
    rx
}

/// Deliver the user's answer. Returns false when nothing was waiting for it — a
/// stray frame from a reconnecting client, which must not become a tool result for
/// some later question.
pub fn answer(session_id: &str, text: &str) -> bool {
    let Ok(mut p) = pending().lock() else { return false };
    match p.remove(session_id) {
        Some(tx) => tx.send(text.to_string()).is_ok(),
        None => false,
    }
}

/// Drop whatever this session was waiting on — the turn ended, was cancelled, or
/// timed out. Called unconditionally when a turn finishes so a question can never
/// outlive the run that asked it.
pub fn abandon(session_id: &str) {
    if let Ok(mut p) = pending().lock() {
        p.remove(session_id);
    }
}

/// Is this session currently blocked on a question?
///
/// Part of the module's surface rather than a helper: "is this run waiting on a
/// person" is the question anything supervising a turn wants to ask. Used by the
/// tests today, which is why the compiler cannot see a caller.
#[allow(dead_code)]
pub fn is_waiting(session_id: &str) -> bool {
    pending().lock().map(|p| p.contains_key(session_id)).unwrap_or(false)
}
