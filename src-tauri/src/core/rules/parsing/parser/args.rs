use super::*;

impl Parser {
    pub(in crate::core::rules) fn args(&mut self, what: String) -> Result<Vec<Expr>, RuleError> {
        let mut out = Vec::new();
        if *self.peek() == Tok::RParen {
            self.bump();
            return Ok(out);
        }
        loop {
            out.push(self.expr()?);
            match self.peek() {
                Tok::Comma => {
                    self.bump();
                }
                Tok::RParen => {
                    self.bump();
                    return Ok(out);
                }
                other => {
                    return Err(RuleError::new(
                        format!("expected `,` or `)` in {what}, found {}", describe(other)),
                        Some(self.at()),
                    ))
                }
            }
        }
    }

    pub(in crate::core::rules) fn primary(&mut self) -> Result<Expr, RuleError> {
        let (tok, at) = self.bump();
        match tok {
            Tok::Str(s) => Ok(Expr::Lit(Val::Str(s))),
            Tok::Int(i) => Ok(Expr::Lit(Val::Int(i))),
            Tok::Float(f) => Ok(Expr::Lit(Val::Float(f))),
            Tok::Ident(name) => match name.as_str() {
                "true" => Ok(Expr::Lit(Val::Bool(true))),
                "false" => Ok(Expr::Lit(Val::Bool(false))),
                "null" => Ok(Expr::Lit(Val::Null)),
                "in" => Err(RuleError::new("`in` needs something on its left, e.g. `intent in [\"send\"]`", Some(at))),
                _ => {
                    if *self.peek() == Tok::LParen {
                        self.bump();
                        let args = self.args(format!("the call to {name}"))?;
                        Ok(Expr::Call(name, args, at))
                    } else {
                        Ok(Expr::Ident(name, at))
                    }
                }
            },
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen, "to close `(`")?;
                Ok(e)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if *self.peek() == Tok::RBracket {
                    self.bump();
                    return Ok(Expr::List(items));
                }
                loop {
                    items.push(self.expr()?);
                    match self.bump() {
                        (Tok::Comma, _) => {}
                        (Tok::RBracket, _) => return Ok(Expr::List(items)),
                        (other, a) => {
                            return Err(RuleError::new(format!("expected `,` or `]` in the list, found {}", describe(&other)), Some(a)))
                        }
                    }
                }
            }
            Tok::End => Err(RuleError::new("the rule ends before it says anything — it needs something after this", Some(at))),
            other => Err(RuleError::new(format!("{} can't start a value here", describe(&other)), Some(at))),
        }
    }
}
