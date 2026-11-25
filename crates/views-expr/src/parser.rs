// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Expression syntax tree and parser.

use crate::{
    ParseError, Value,
    lexer::{Tok, Token, lex},
};

/// Deepest nesting the parser accepts. Keeps hostile input off the stack.
pub const MAX_DEPTH: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Coalesce,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(Value),
    Var(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    Lambda(Vec<String>, Box<Expr>),
}

impl Expr {
    /// The static path this expression names (`shared.items.0`), if it is one.
    pub fn as_path(&self) -> Option<Vec<String>> {
        match self {
            Expr::Var(v) => Some(vec![v.clone()]),
            Expr::Member(b, f) => {
                let mut p = b.as_path()?;
                p.push(f.clone());
                Some(p)
            }
            Expr::Index(b, i) => {
                let mut p = b.as_path()?;
                match &**i {
                    Expr::Lit(Value::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => {
                        p.push((*n as usize).to_string())
                    }
                    Expr::Lit(Value::Str(s)) => p.push(s.to_string()),
                    _ => return None,
                }
                Some(p)
            }
            _ => None,
        }
    }
}

/// Parse one expression.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
        depth: 0,
    };
    let e = p.expr()?;
    p.expect_eof()?;
    Ok(e)
}

/// Parse a `;`-separated sequence of expressions (an action list).
pub fn parse_seq(src: &str) -> Result<Vec<Expr>, ParseError> {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
        depth: 0,
    };
    let mut out = Vec::new();
    loop {
        while p.eat(";") {}
        if p.peek() == &Tok::Eof {
            break;
        }
        out.push(p.expr()?);
        if !p.eat(";") {
            p.expect_eof()?;
            break;
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }

    fn offset(&self) -> usize {
        self.toks[self.pos].start
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, p: &str) -> bool {
        if matches!(self.peek(), Tok::Punct(q) if *q == p) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        if self.peek() == &Tok::Eof {
            ParseError::incomplete(msg, self.offset())
        } else {
            ParseError::new(msg, self.offset())
        }
    }

    fn expect(&mut self, p: &str) -> Result<(), ParseError> {
        if self.eat(p) {
            Ok(())
        } else {
            Err(self.err(format!("expected '{p}'")))
        }
    }

    fn expect_eof(&self) -> Result<(), ParseError> {
        if self.peek() == &Tok::Eof {
            Ok(())
        } else {
            Err(self.err("unexpected input after expression"))
        }
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.bump();
                Ok(s)
            }
            _ => Err(self.err("expected a name")),
        }
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ParseError::new(
                "expression nested too deeply",
                self.offset(),
            ));
        }
        let r = self.lambda_or_cond();
        self.depth -= 1;
        r
    }

    fn lambda_or_cond(&mut self) -> Result<Expr, ParseError> {
        if let (Tok::Ident(name), Tok::Punct("=>")) = (self.peek().clone(), self.peek_at(1)) {
            self.bump();
            self.bump();
            return Ok(Expr::Lambda(vec![name], Box::new(self.expr()?)));
        }
        if self.peek() == &Tok::Punct("(") && self.is_paren_lambda() {
            self.bump();
            let mut params = Vec::new();
            while !self.eat(")") {
                params.push(self.ident()?);
                if !self.eat(",") {
                    self.expect(")")?;
                    break;
                }
            }
            self.expect("=>")?;
            return Ok(Expr::Lambda(params, Box::new(self.expr()?)));
        }
        let c = self.binary(0)?;
        if self.eat("?") {
            let a = self.expr()?;
            self.expect(":")?;
            let b = self.expr()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn is_paren_lambda(&self) -> bool {
        let mut i = 1;
        loop {
            match self.peek_at(i) {
                Tok::Ident(_) | Tok::Punct(",") => i += 1,
                Tok::Punct(")") => return self.peek_at(i + 1) == &Tok::Punct("=>"),
                _ => return false,
            }
        }
    }

    fn binary(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        while let Tok::Punct(p) = self.peek() {
            let Some((op, prec)) = binop(p) else { break };
            if prec < min_prec {
                break;
            }
            self.bump();
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(ParseError::new(
                    "expression nested too deeply",
                    self.offset(),
                ));
            }
            let rhs = self.binary(prec + 1);
            self.depth -= 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs?));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        let op = if self.eat("!") {
            UnOp::Not
        } else if self.eat("-") {
            UnOp::Neg
        } else {
            return self.postfix();
        };
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ParseError::new(
                "expression nested too deeply",
                self.offset(),
            ));
        }
        let e = self.unary();
        self.depth -= 1;
        Ok(Expr::Unary(op, Box::new(e?)))
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.primary()?;
        loop {
            if self.eat(".") {
                e = Expr::Member(Box::new(e), self.ident()?);
            } else if self.eat("[") {
                let i = self.expr()?;
                self.expect("]")?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.peek() == &Tok::Punct("(") {
                let Expr::Var(name) = e else {
                    return Err(self.err("only named functions can be called"));
                };
                self.bump();
                let args = self.list_items(")")?;
                e = Expr::Call(name, args);
            } else {
                return Ok(e);
            }
        }
    }

    fn list_items(&mut self, close: &str) -> Result<Vec<Expr>, ParseError> {
        let mut items = Vec::new();
        while !self.eat(close) {
            items.push(self.expr()?);
            if !self.eat(",") {
                self.expect(close)?;
                break;
            }
        }
        Ok(items)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Tok::Num(n) => {
                self.bump();
                Ok(Expr::Lit(Value::Num(n)))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr::Lit(Value::str(s)))
            }
            Tok::Ident(s) => {
                self.bump();
                Ok(match s.as_str() {
                    "true" => Expr::Lit(Value::Bool(true)),
                    "false" => Expr::Lit(Value::Bool(false)),
                    "null" => Expr::Lit(Value::Null),
                    _ => Expr::Var(s),
                })
            }
            Tok::Punct("(") => {
                self.bump();
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Punct("[") => {
                self.bump();
                Ok(Expr::List(self.list_items("]")?))
            }
            Tok::Punct("{") => {
                self.bump();
                let mut fields = Vec::new();
                while !self.eat("}") {
                    let key = match self.bump() {
                        Tok::Ident(s) | Tok::Str(s) => s,
                        Tok::Eof => {
                            return Err(ParseError::incomplete("expected '}'", self.offset()));
                        }
                        _ => return Err(ParseError::new("expected a field name", self.offset())),
                    };
                    self.expect(":")?;
                    fields.push((key, self.expr()?));
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                }
                Ok(Expr::Record(fields))
            }
            Tok::Eof => Err(self.err("expected an expression")),
            other => Err(self.err(format!("unexpected {}", describe(&other)))),
        }
    }
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Punct(p) => format!("'{p}'"),
        Tok::Num(_) => "number".into(),
        Tok::Str(_) => "text".into(),
        Tok::Ident(s) => format!("'{s}'"),
        Tok::Eof => "end".into(),
    }
}

fn binop(p: &str) -> Option<(BinOp, u8)> {
    Some(match p {
        "??" => (BinOp::Coalesce, 1),
        "||" => (BinOp::Or, 2),
        "&&" => (BinOp::And, 3),
        "==" => (BinOp::Eq, 4),
        "!=" => (BinOp::Ne, 4),
        "<" => (BinOp::Lt, 5),
        "<=" => (BinOp::Le, 5),
        ">" => (BinOp::Gt, 5),
        ">=" => (BinOp::Ge, 5),
        "+" => (BinOp::Add, 6),
        "-" => (BinOp::Sub, 6),
        "*" => (BinOp::Mul, 7),
        "/" => (BinOp::Div, 7),
        "%" => (BinOp::Rem, 7),
        _ => return None,
    })
}
