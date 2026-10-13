use super::super::*;

pub(in crate::core::rules) fn binary(op: Bin, a: Val, b: Val, at: usize) -> Result<Val, RuleError> {
    let order = |a: &Val, b: &Val| -> Result<std::cmp::Ordering, RuleError> {
        match (a, b) {
            (Val::Str(x), Val::Str(y)) => Ok(x.cmp(y)),
            _ => match (number(a), number(b)) {
                (Some(x), Some(y)) => x.partial_cmp(&y).ok_or_else(|| RuleError::new("those numbers can't be compared", Some(at))),
                _ => Err(RuleError::new(format!("can't compare {} with {}", a.kind(), b.kind()), Some(at))),
            },
        }
    };
    use std::cmp::Ordering::*;
    Ok(match op {
        Bin::Eq => Val::Bool(equal(&a, &b)),
        Bin::Ne => Val::Bool(!equal(&a, &b)),
        Bin::Lt => Val::Bool(order(&a, &b)? == Less),
        Bin::Le => Val::Bool(order(&a, &b)? != Greater),
        Bin::Gt => Val::Bool(order(&a, &b)? == Greater),
        Bin::Ge => Val::Bool(order(&a, &b)? != Less),
        Bin::In => match &b {
            Val::List(items) => Val::Bool(items.iter().any(|i| equal(i, &a))),
            Val::Map(m) => match &a {
                Val::Str(k) => Val::Bool(m.contains_key(k)),
                _ => Val::Bool(false),
            },
            other => {
                return Err(RuleError::new(
                    format!("`in` looks inside a list, but got {} — to search text, use contains(text, \"…\")", other.kind()),
                    Some(at),
                ))
            }
        },
        Bin::Add => match (a, b) {
            (Val::Int(x), Val::Int(y)) => Val::Int(x.checked_add(y).ok_or_else(|| RuleError::new("that number is too large", Some(at)))?),
            (Val::Str(x), Val::Str(y)) => Val::Str(x + &y),
            (Val::List(mut x), Val::List(y)) => {
                x.extend(y);
                Val::List(x)
            }
            (a, b) => match (number(&a), number(&b)) {
                (Some(x), Some(y)) => Val::Float(x + y),
                _ => return Err(RuleError::new(format!("can't add {} and {}", a.kind(), b.kind()), Some(at))),
            },
        },
        Bin::Sub | Bin::Mul | Bin::Div | Bin::Rem => match (&a, &b) {
            (Val::Int(x), Val::Int(y)) => {
                let r = match op {
                    Bin::Sub => x.checked_sub(*y),
                    Bin::Mul => x.checked_mul(*y),
                    Bin::Div => x.checked_div(*y),
                    _ => x.checked_rem(*y),
                };
                Val::Int(r.ok_or_else(|| RuleError::new("that arithmetic has no answer (a division by zero, or too large)", Some(at)))?)
            }
            _ => match (number(&a), number(&b)) {
                (Some(x), Some(y)) => Val::Float(match op {
                    Bin::Sub => x - y,
                    Bin::Mul => x * y,
                    Bin::Div if y != 0.0 => x / y,
                    Bin::Rem if y != 0.0 => x % y,
                    _ => return Err(RuleError::new("that arithmetic divides by zero", Some(at))),
                }),
                _ => return Err(RuleError::new(format!("that arithmetic needs numbers, but got {} and {}", a.kind(), b.kind()), Some(at))),
            },
        },
    })
}

impl Program {
    /// The value this rule computes for these facts.
    pub fn eval(&self, ctx: &Json) -> Result<Val, RuleError> {
        eval(&self.expr, &Val::from_json(ctx))
    }

    /// Does this rule apply? An answer that is not true or false is an error —
    /// `"Submit order"` is a perfectly valid expression that answers nothing, and
    /// reading it as "no" would let a deny list quietly stop denying.
    pub fn test(&self, ctx: &Json) -> Result<bool, RuleError> {
        match self.eval(ctx)? {
            Val::Bool(b) => Ok(b),
            other => Err(RuleError::new(
                format!("a rule has to answer true or false, and this one answers {} — did you mean to compare it with something?", other.kind()),
                None,
            )),
        }
    }

    /// Every field path the rule reads (`element.name`, `intent`), for checking a
    /// rule against the fields that actually exist before it is saved.
    pub fn fields(&self) -> Vec<(String, usize)> {
        fn path(e: &Expr) -> Option<(String, usize)> {
            match e {
                Expr::Ident(n, at) => Some((n.clone(), *at)),
                Expr::Member(base, f, _) => path(base).map(|(p, at)| (format!("{p}.{f}"), at)),
                _ => None,
            }
        }
        fn walk(e: &Expr, out: &mut Vec<(String, usize)>) {
            if let Some(p) = path(e) {
                if !out.iter().any(|(q, _)| *q == p.0) {
                    out.push(p);
                }
                return;
            }
            match e {
                Expr::Lit(_) | Expr::Ident(..) => {}
                Expr::Member(b, _, _) | Expr::Not(b, _) | Expr::Neg(b, _) => walk(b, out),
                Expr::Index(b, i, _) => {
                    walk(b, out);
                    walk(i, out);
                }
                Expr::Call(_, args, _) | Expr::List(args) => args.iter().for_each(|a| walk(a, out)),
                Expr::Method(r, _, args, _) => {
                    walk(r, out);
                    args.iter().for_each(|a| walk(a, out));
                }
                Expr::Binary(_, l, r, _) | Expr::And(l, r) | Expr::Or(l, r) => {
                    walk(l, out);
                    walk(r, out);
                }
                Expr::Cond(c, y, n, _) => {
                    walk(c, out);
                    walk(y, out);
                    walk(n, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.expr, &mut out);
        out
    }
}
