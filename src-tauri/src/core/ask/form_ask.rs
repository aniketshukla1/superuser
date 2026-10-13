use super::*;

impl Form {
    pub fn parse(args: &Value) -> Result<Self, String> {
        let raw = args["fields"].as_array().cloned().unwrap_or_default();
        let mut fields = Vec::new();
        for f in raw.iter().take(MAX_FIELDS) {
            let label = f["label"].as_str().unwrap_or("").trim().to_string();
            if label.is_empty() {
                continue;
            }
            // A name is what the answer comes back under. Derived from the label when
            // absent, because a model that wrote a good label has said enough.
            let name = f["name"]
                .as_str()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    label
                        .to_lowercase()
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                        .trim_matches('_')
                        .to_string()
                });
            let kind = match f["kind"].as_str().unwrap_or("text").trim() {
                k @ ("text" | "number" | "date") => k,
                _ => "text",
            }
            .to_string();
            fields.push(Field {
                name,
                label,
                kind,
                placeholder: f["placeholder"].as_str().unwrap_or("").trim().to_string(),
                help: f["help"].as_str().unwrap_or("").trim().to_string(),
                // Required unless it says otherwise: a field the agent did not need is
                // a field it should not have asked for.
                required: f["required"].as_bool().unwrap_or(true),
            });
        }
        if fields.is_empty() {
            return Err(
                "error: `fields` must hold at least one field with a `label`. If what you need is a \
                 DECISION rather than a value, use `ask_user` and give the options you think are likely."
                    .into(),
            );
        }
        if raw.len() > MAX_FIELDS {
            return Err(format!(
                "error: {} fields is an intake form, not a question. Ask for the {MAX_FIELDS} you cannot \
                 proceed without, and find the rest.",
                raw.len()
            ));
        }
        Ok(Form {
            title: args["title"].as_str().unwrap_or("").trim().to_string(),
            intro: args["intro"].as_str().unwrap_or("").trim().to_string(),
            fields,
        })
    }

    pub fn summary(&self) -> String {
        if !self.title.is_empty() {
            return self.title.clone();
        }
        match self.fields.len() {
            1 => self.fields[0].label.clone(),
            n => format!("{} (and {} more)", self.fields[0].label, n - 1),
        }
    }

    pub fn payload(&self) -> Value {
        json!({ "title": self.title, "intro": self.intro, "fields": self.fields })
    }

    /// Turn the card's reply into what the model reads.
    ///
    /// Same rule as `Ask::read_answer`: a field left blank is reported as blank in as
    /// many words. Dropping it would hand back a shorter list than was asked, which
    /// reads as "that field didn't happen" — and the gap is where a model fills in a
    /// guess.
    pub fn read_answer(&self, raw: &str) -> String {
        let parsed: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
        // `{"answers": {...}}` from the card; a bare object is tolerated too.
        let map = parsed
            .get("answers")
            .filter(|v| v.is_object())
            .or(Some(&parsed))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        if map.is_empty() {
            return format!("The user answered: {}", raw.trim());
        }
        let mut lines = Vec::new();
        let mut blank = 0;
        for f in &self.fields {
            let v = map
                .get(&f.name)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.trim().to_string()),
                    Value::Null => None,
                    other => Some(other.to_string()),
                })
                .filter(|s| !s.is_empty());
            match v {
                Some(v) => lines.push(format!("{} → {v}", f.label)),
                None => {
                    blank += 1;
                    lines.push(format!("{} → (left blank)", f.label));
                }
            }
        }
        let mut out = format!("The user filled this in:\n{}", lines.join("\n"));
        if blank > 0 {
            out.push_str(
                "\n\nDo not invent a value for anything left blank. Either proceed without it and say so, \
                 or ask once more for the one you actually cannot continue without.",
            );
        }
        out
    }
}

/// Most questions in one ask.
///
/// Four. Past that it stops being a clarification and becomes an intake form, and a
/// model reaching for eight has usually not worked out which two actually matter.
pub(super) const MAX_QUESTIONS: usize = 4;

/// Most options on one question, before "Other".
///
/// Five with the escape, which is the point at which a list stops being scannable and
/// the number-key shortcuts run out of one-digit keys.
pub(super) const MAX_OPTIONS: usize = 4;

/// Fewest options that make a question answerable in a click rather than a sentence.
pub(super) const MIN_OPTIONS: usize = 2;

pub(super) fn choices_of(v: &Value) -> Vec<Choice> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|o| {
                    // Models reach for a bare string as often as the documented object,
                    // and rejecting the simple form would burn a turn on a reformat.
                    let (label, description) = match o {
                        Value::String(s) => (s.trim().to_string(), String::new()),
                        _ => (
                            o["label"].as_str().unwrap_or("").trim().to_string(),
                            o["description"].as_str().unwrap_or("").trim().to_string(),
                        ),
                    };
                    (!label.is_empty()).then_some(Choice { label, description })
                })
                .take(MAX_OPTIONS)
                .collect()
        })
        .unwrap_or_default()
}

impl Question {
    pub(super) fn parse(v: &Value) -> Option<Result<Self, String>> {
        let question = v["question"].as_str().unwrap_or("").trim().to_string();
        if question.is_empty() {
            return None;
        }
        let options = choices_of(&v["options"]);
        if options.len() < MIN_OPTIONS {
            return Some(Err(format!(
                "error: \"{}\" has fewer than {MIN_OPTIONS} options. Every question needs the answers you \
                 think are likely, each with a short line saying what it means — writing them is the work, \
                 and it lets the user answer in one click instead of composing a reply. If you cannot name \
                 two plausible answers, you do not understand the question well enough to ask it yet: go and \
                 look first. The card always adds a free-text \"Other\", so you never need to.",
                question.chars().take(60).collect::<String>()
            )));
        }
        Some(Ok(Question { question, options, multi: v["multi"].as_bool().unwrap_or(false) }))
    }
}
