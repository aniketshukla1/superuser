use super::super::*;

pub(in crate::core::rules) fn lex(src: &str) -> Result<Vec<(Tok, usize)>, RuleError> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        // Raw strings first: `r"..."` would otherwise lex as the identifier `r`.
        if (c == 'r' || c == 'R') && matches!(chars.get(i + 1), Some('"') | Some('\'')) {
            let quote = chars[i + 1];
            i += 2;
            let mut s = String::new();
            loop {
                match chars.get(i) {
                    None => return Err(RuleError::new("this text is never closed — add the closing quote", Some(start))),
                    Some(ch) if *ch == quote => break,
                    Some(ch) => s.push(*ch),
                }
                i += 1;
            }
            i += 1;
            out.push((Tok::Str(s), start));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let mut s = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                s.push(chars[i]);
                i += 1;
            }
            out.push((Tok::Ident(s), start));
            continue;
        }
        if c.is_ascii_digit() {
            let mut s = String::new();
            let mut float = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || (chars[i] == '.' && !float && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))) {
                if chars[i] == '.' {
                    float = true;
                }
                s.push(chars[i]);
                i += 1;
            }
            let tok = if float {
                Tok::Float(s.parse().map_err(|_| RuleError::new("that is not a number", Some(start)))?)
            } else {
                Tok::Int(s.parse().map_err(|_| RuleError::new("that number is too large", Some(start)))?)
            };
            out.push((tok, start));
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            loop {
                match chars.get(i) {
                    None => return Err(RuleError::new("this text is never closed — add the closing quote", Some(start))),
                    Some(ch) if *ch == quote => break,
                    Some('\\') => {
                        let esc = chars.get(i + 1).copied();
                        match esc {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('r') => s.push('\r'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some('\'') => s.push('\''),
                            // Kept as written, so a regular expression typed inside
                            // ordinary quotes (`"\d+"`) still means what it says.
                            Some(other) => {
                                s.push('\\');
                                s.push(other);
                            }
                            None => return Err(RuleError::new("this text is never closed — add the closing quote", Some(start))),
                        }
                        i += 1;
                    }
                    Some(ch) => s.push(*ch),
                }
                i += 1;
            }
            i += 1;
            out.push((Tok::Str(s), start));
            continue;
        }
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let tok2 = match two.as_str() {
            "&&" => Some(Tok::AndAnd),
            "||" => Some(Tok::OrOr),
            "==" => Some(Tok::EqEq),
            "!=" => Some(Tok::NotEq),
            "<=" => Some(Tok::Le),
            ">=" => Some(Tok::Ge),
            _ => None,
        };
        if let Some(t) = tok2 {
            out.push((t, start));
            i += 2;
            continue;
        }
        let tok = match c {
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ',' => Tok::Comma,
            '.' => Tok::Dot,
            '!' => Tok::Bang,
            '-' => Tok::Minus,
            '+' => Tok::Plus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '%' => Tok::Percent,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '?' => Tok::Question,
            ':' => Tok::Colon,
            // The two mistakes people coming from other languages actually make.
            '=' => return Err(RuleError::new("a single `=` is not a comparison — write `==`", Some(start))),
            '&' => return Err(RuleError::new("write `&&` for \"and\"", Some(start))),
            '|' => return Err(RuleError::new("write `||` for \"or\"", Some(start))),
            other => return Err(RuleError::new(format!("`{other}` can't be used in a rule"), Some(start))),
        };
        out.push((tok, start));
        i += 1;
    }
    out.push((Tok::End, chars.len()));
    Ok(out)
}
