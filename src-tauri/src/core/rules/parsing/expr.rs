use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::core::rules) enum Bin {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone)]
pub(in crate::core::rules) enum Expr {
    Lit(Val),
    Ident(String, usize),
    Member(Box<Expr>, String, usize),
    Index(Box<Expr>, Box<Expr>, usize),
    Call(String, Vec<Expr>, usize),
    Method(Box<Expr>, String, Vec<Expr>, usize),
    List(Vec<Expr>),
    Not(Box<Expr>, usize),
    Neg(Box<Expr>, usize),
    Binary(Bin, Box<Expr>, Box<Expr>, usize),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>, usize),
}

pub(in crate::core::rules) struct Parser {
    pub(in crate::core::rules) toks: Vec<(Tok, usize)>,
    pub(in crate::core::rules) pos: usize,
    pub(in crate::core::rules) depth: usize,
}
