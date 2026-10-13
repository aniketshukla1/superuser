use super::super::*;

/// A rule, read once and answerable many times.
#[derive(Debug, Clone)]
pub struct Program {
    pub(in crate::core::rules) expr: Expr,
}

/// Read a rule. The error says what is wrong and where, in words meant for the
/// person who typed it.
pub fn parse(src: &str) -> Result<Program, RuleError> {
    if src.chars().count() > MAX_LEN {
        return Err(RuleError::new(format!("a rule may be at most {MAX_LEN} characters"), None));
    }
    if src.trim().is_empty() {
        return Err(RuleError::new("the rule is empty", None));
    }
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0, depth: 0 };
    let expr = p.expr()?;
    if *p.peek() != Tok::End {
        return Err(RuleError::new(
            format!("the rule should have ended here, but continues with {}", describe(p.peek())),
            Some(p.at()),
        ));
    }
    Ok(Program { expr })
}
