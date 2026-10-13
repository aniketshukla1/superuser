use super::*;

/// The longest rule accepted. A boundary is a sentence, not a program, and a bound
/// keeps a pasted novel from becoming a per-tool-call parsing cost.
pub const MAX_LEN: usize = 2000;

/// How deep an expression may nest before it is refused. Parsing is recursive, and
/// `((((…))))` a few thousand deep would otherwise exhaust the stack of whichever
/// thread happened to be deciding a tool call.
pub(super) const MAX_DEPTH: usize = 64;

/// A value a rule computes with.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Val>),
    Map(BTreeMap<String, Val>),
}

impl Val {
    pub fn from_json(v: &Json) -> Val {
        match v {
            Json::Null => Val::Null,
            Json::Bool(b) => Val::Bool(*b),
            Json::Number(n) => match n.as_i64() {
                Some(i) => Val::Int(i),
                None => Val::Float(n.as_f64().unwrap_or(0.0)),
            },
            Json::String(s) => Val::Str(s.clone()),
            Json::Array(a) => Val::List(a.iter().map(Val::from_json).collect()),
            Json::Object(o) => Val::Map(o.iter().map(|(k, v)| (k.clone(), Val::from_json(v))).collect()),
        }
    }

    /// What kind of thing this is, in the words an error message uses.
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Val::Null => "null",
            Val::Bool(_) => "true or false",
            Val::Int(_) | Val::Float(_) => "a number",
            Val::Str(_) => "text",
            Val::List(_) => "a list",
            Val::Map(_) => "a group of fields",
        }
    }
}

/// Why a rule could not be read or could not be answered.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleError {
    pub message: String,
    /// Character offset into the rule, when the problem has a place.
    pub at: Option<usize>,
}

impl RuleError {
    pub(super) fn new(message: impl Into<String>, at: Option<usize>) -> Self {
        RuleError { message: message.into(), at }
    }
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.at {
            // 1-based, because a person counts columns from one.
            Some(at) => write!(f, "at character {}: {}", at + 1, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}
