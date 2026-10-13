use super::*;

impl Parser {
    pub(in crate::core::rules) fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    pub(in crate::core::rules) fn at(&self) -> usize {
        self.toks[self.pos].1
    }
    pub(in crate::core::rules) fn bump(&mut self) -> (Tok, usize) {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    pub(in crate::core::rules) fn expect(&mut self, want: Tok, context: &str) -> Result<(), RuleError> {
        if *self.peek() == want {
            self.bump();
            Ok(())
        } else {
            Err(RuleError::new(
                format!("expected {} {context}, found {}", describe(&want), describe(self.peek())),
                Some(self.at()),
            ))
        }
    }
    pub(in crate::core::rules) fn enter(&mut self) -> Result<(), RuleError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(RuleError::new("this rule nests too deeply", Some(self.at())));
        }
        Ok(())
    }

    pub(in crate::core::rules) fn expr(&mut self) -> Result<Expr, RuleError> {
        self.enter()?;
        let cond = self.or()?;
        let out = if *self.peek() == Tok::Question {
            let at = self.at();
            self.bump();
            let yes = self.expr()?;
            self.expect(Tok::Colon, "between the two answers of `? :`")?;
            let no = self.expr()?;
            Expr::Cond(Box::new(cond), Box::new(yes), Box::new(no), at)
        } else {
            cond
        };
        self.depth -= 1;
        Ok(out)
    }

    pub(in crate::core::rules) fn or(&mut self) -> Result<Expr, RuleError> {
        let mut left = self.and()?;
        while *self.peek() == Tok::OrOr {
            self.bump();
            let right = self.and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    pub(in crate::core::rules) fn and(&mut self) -> Result<Expr, RuleError> {
        let mut left = self.relation()?;
        while *self.peek() == Tok::AndAnd {
            self.bump();
            let right = self.relation()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    pub(in crate::core::rules) fn relation(&mut self) -> Result<Expr, RuleError> {
        let left = self.additive()?;
        let at = self.at();
        let op = match self.peek() {
            Tok::EqEq => Bin::Eq,
            Tok::NotEq => Bin::Ne,
            Tok::Lt => Bin::Lt,
            Tok::Le => Bin::Le,
            Tok::Gt => Bin::Gt,
            Tok::Ge => Bin::Ge,
            Tok::Ident(s) if s == "in" => Bin::In,
            _ => return Ok(left),
        };
        self.bump();
        let right = self.additive()?;
        // `a == b == c` compares a boolean with c, which is never what was meant.
        if matches!(self.peek(), Tok::EqEq | Tok::NotEq | Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge) {
            return Err(RuleError::new("two comparisons in a row need `&&` between them", Some(self.at())));
        }
        Ok(Expr::Binary(op, Box::new(left), Box::new(right), at))
    }

    pub(in crate::core::rules) fn additive(&mut self) -> Result<Expr, RuleError> {
        let mut left = self.multiplicative()?;
        loop {
            let at = self.at();
            let op = match self.peek() {
                Tok::Plus => Bin::Add,
                Tok::Minus => Bin::Sub,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right), at);
        }
    }

    pub(in crate::core::rules) fn multiplicative(&mut self) -> Result<Expr, RuleError> {
        let mut left = self.unary()?;
        loop {
            let at = self.at();
            let op = match self.peek() {
                Tok::Star => Bin::Mul,
                Tok::Slash => Bin::Div,
                Tok::Percent => Bin::Rem,
                _ => return Ok(left),
            };
            self.bump();
            let right = self.unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right), at);
        }
    }

    pub(in crate::core::rules) fn unary(&mut self) -> Result<Expr, RuleError> {
        self.enter()?;
        let at = self.at();
        let out = match self.peek() {
            Tok::Bang => {
                self.bump();
                Expr::Not(Box::new(self.unary()?), at)
            }
            Tok::Minus => {
                self.bump();
                Expr::Neg(Box::new(self.unary()?), at)
            }
            _ => self.member()?,
        };
        self.depth -= 1;
        Ok(out)
    }

    pub(in crate::core::rules) fn member(&mut self) -> Result<Expr, RuleError> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.bump();
                    let at = self.at();
                    let name = match self.bump().0 {
                        Tok::Ident(n) => n,
                        other => {
                            return Err(RuleError::new(format!("expected a field name after `.`, found {}", describe(&other)), Some(at)))
                        }
                    };
                    if *self.peek() == Tok::LParen {
                        self.bump();
                        let args = self.args("the call to ".to_string() + &name)?;
                        e = Expr::Method(Box::new(e), name, args, at);
                    } else {
                        e = Expr::Member(Box::new(e), name, at);
                    }
                }
                Tok::LBracket => {
                    let at = self.at();
                    self.bump();
                    let idx = self.expr()?;
                    self.expect(Tok::RBracket, "to close `[`")?;
                    e = Expr::Index(Box::new(e), Box::new(idx), at);
                }
                _ => return Ok(e),
            }
        }
    }
}
