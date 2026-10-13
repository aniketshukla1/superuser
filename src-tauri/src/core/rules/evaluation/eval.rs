use super::super::*;

pub(in crate::core::rules) fn lower(s: &str) -> String {
    s.to_lowercase()
}

pub(in crate::core::rules) fn want_str<'a>(v: &'a Val, what: &str, at: usize) -> Result<&'a str, RuleError> {
    match v {
        Val::Str(s) => Ok(s),
        other => Err(RuleError::new(format!("{what} needs text, but got {}", other.kind()), Some(at))),
    }
}

pub(in crate::core::rules) fn want_bool(v: Val, what: &str, at: Option<usize>) -> Result<bool, RuleError> {
    match v {
        Val::Bool(b) => Ok(b),
        other => Err(RuleError::new(format!("{what} needs true or false, but got {}", other.kind()), at)),
    }
}

pub(in crate::core::rules) fn number(v: &Val) -> Option<f64> {
    match v {
        Val::Int(i) => Some(*i as f64),
        Val::Float(f) => Some(*f),
        _ => None,
    }
}

/// CEL's equality: values of different kinds are simply unequal, except that an
/// integer and a decimal compare by value. Never an error — "is it this?" always has
/// an answer.
pub(in crate::core::rules) fn equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Int(_) | Val::Float(_), Val::Int(_) | Val::Float(_)) => number(a) == number(b),
        _ => a == b,
    }
}

/// The functions a rule can call, by name. One table for both spellings —
/// `contains(x, y)` and `x.contains(y)` — so the two can never drift apart.
pub(in crate::core::rules) fn call(name: &str, args: Vec<Val>, at: usize) -> Result<Val, RuleError> {
    let arity = |n: usize| -> Result<(), RuleError> {
        if args.len() == n {
            Ok(())
        } else {
            Err(RuleError::new(format!("{name} takes {n} value{}, not {}", if n == 1 { "" } else { "s" }, args.len()), Some(at)))
        }
    };
    match name {
        // Case-insensitive: a rule about "submit" must also stop
        // "SUBMIT", or the rule that looks right is the one that leaks.
        "contains" => {
            arity(2)?;
            match (&args[0], &args[1]) {
                (Val::Str(h), Val::Str(n)) => Ok(Val::Bool(lower(h).contains(&lower(n)))),
                (Val::List(items), needle) => Ok(Val::Bool(items.iter().any(|i| match (i, needle) {
                    (Val::Str(a), Val::Str(b)) => lower(a) == lower(b),
                    _ => equal(i, needle),
                }))),
                (h, _) => Err(RuleError::new(format!("contains needs text or a list to look in, but got {}", h.kind()), Some(at))),
            }
        }
        "startsWith" | "endsWith" => {
            arity(2)?;
            let s = lower(want_str(&args[0], name, at)?);
            let p = lower(want_str(&args[1], name, at)?);
            Ok(Val::Bool(if name == "startsWith" { s.starts_with(&p) } else { s.ends_with(&p) }))
        }
        "matches" => {
            arity(2)?;
            let s = want_str(&args[0], name, at)?;
            let pattern = want_str(&args[1], name, at)?;
            // A pattern that cannot be compiled is a broken rule, never a quiet
            // "no match" — which on a deny list would be a rule that silently stopped
            // denying. Bounded so a pathological pattern costs a refusal, not the app.
            let re = regex::RegexBuilder::new(pattern)
                .case_insensitive(true)
                .size_limit(1 << 20)
                .build()
                .map_err(|_| RuleError::new(format!("`{pattern}` is not a pattern matches can read"), Some(at)))?;
            Ok(Val::Bool(re.is_match(s)))
        }
        "size" => {
            arity(1)?;
            match &args[0] {
                Val::Str(s) => Ok(Val::Int(s.chars().count() as i64)),
                Val::List(l) => Ok(Val::Int(l.len() as i64)),
                Val::Map(m) => Ok(Val::Int(m.len() as i64)),
                other => Err(RuleError::new(format!("size needs text or a list, but got {}", other.kind()), Some(at))),
            }
        }
        "lower" | "lowerAscii" => {
            arity(1)?;
            Ok(Val::Str(lower(want_str(&args[0], name, at)?)))
        }
        other => Err(RuleError::new(
            format!("there is no function called {other} — rules can use contains, matches, startsWith, endsWith, size and lower"),
            Some(at),
        )),
    }
}

pub(in crate::core::rules) fn eval(e: &Expr, ctx: &Val) -> Result<Val, RuleError> {
    match e {
        Expr::Lit(v) => Ok(v.clone()),
        Expr::Ident(name, at) => match ctx {
            Val::Map(m) => m
                .get(name)
                .cloned()
                .ok_or_else(|| RuleError::new(format!("there is no field called `{name}`"), Some(*at))),
            _ => Err(RuleError::new(format!("there is no field called `{name}`"), Some(*at))),
        },
        Expr::Member(base, field, at) => match eval(base, ctx)? {
            Val::Map(m) => m
                .get(field)
                .cloned()
                .ok_or_else(|| RuleError::new(format!("there is no field called `{field}` here"), Some(*at))),
            other => Err(RuleError::new(format!("`.{field}` needs a group of fields, but got {}", other.kind()), Some(*at))),
        },
        Expr::Index(base, idx, at) => {
            let b = eval(base, ctx)?;
            let i = eval(idx, ctx)?;
            match (b, i) {
                (Val::List(l), Val::Int(n)) => usize::try_from(n)
                    .ok()
                    .and_then(|n| l.get(n).cloned())
                    .ok_or_else(|| RuleError::new(format!("the list has no item {n}"), Some(*at))),
                (Val::Map(m), Val::Str(k)) => {
                    m.get(&k).cloned().ok_or_else(|| RuleError::new(format!("there is no field called `{k}` here"), Some(*at)))
                }
                (b, _) => Err(RuleError::new(format!("`[ ]` can't be used on {}", b.kind()), Some(*at))),
            }
        }
        Expr::Call(name, args, at) => {
            let vals = args.iter().map(|a| eval(a, ctx)).collect::<Result<Vec<_>, _>>()?;
            call(name, vals, *at)
        }
        Expr::Method(recv, name, args, at) => {
            let mut vals = vec![eval(recv, ctx)?];
            for a in args {
                vals.push(eval(a, ctx)?);
            }
            call(name, vals, *at)
        }
        Expr::List(items) => Ok(Val::List(items.iter().map(|i| eval(i, ctx)).collect::<Result<_, _>>()?)),
        Expr::Not(inner, at) => Ok(Val::Bool(!want_bool(eval(inner, ctx)?, "`!`", Some(*at))?)),
        Expr::Neg(inner, at) => match eval(inner, ctx)? {
            Val::Int(i) => Ok(Val::Int(-i)),
            Val::Float(f) => Ok(Val::Float(-f)),
            other => Err(RuleError::new(format!("`-` needs a number, but got {}", other.kind()), Some(*at))),
        },
        // CEL's `&&` and `||` absorb an error when the other side already decides the
        // answer, in either order: `false && broken` is false and so is
        // `broken && false`. Evaluating both sides before giving up is what makes the
        // result not depend on which way round the person happened to write it.
        Expr::And(l, r) => {
            let left = eval(l, ctx).and_then(|v| want_bool(v, "`&&`", None));
            if left == Ok(false) {
                return Ok(Val::Bool(false));
            }
            let right = eval(r, ctx).and_then(|v| want_bool(v, "`&&`", None));
            if right == Ok(false) {
                return Ok(Val::Bool(false));
            }
            Ok(Val::Bool(left? && right?))
        }
        Expr::Or(l, r) => {
            let left = eval(l, ctx).and_then(|v| want_bool(v, "`||`", None));
            if left == Ok(true) {
                return Ok(Val::Bool(true));
            }
            let right = eval(r, ctx).and_then(|v| want_bool(v, "`||`", None));
            if right == Ok(true) {
                return Ok(Val::Bool(true));
            }
            Ok(Val::Bool(left? || right?))
        }
        Expr::Cond(c, yes, no, at) => {
            if want_bool(eval(c, ctx)?, "`? :`", Some(*at))? {
                eval(yes, ctx)
            } else {
                eval(no, ctx)
            }
        }
        Expr::Binary(op, l, r, at) => {
            let a = eval(l, ctx)?;
            let b = eval(r, ctx)?;
            binary(*op, a, b, *at)
        }
    }
}
