// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tokens.

use crate::ParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

const PUNCT: &[&str] = &[
    "=>", "==", "!=", "<=", ">=", "&&", "||", "??", "(", ")", "[", "]", "{", "}", ",", ".", ":",
    "?", ";", "+", "-", "*", "/", "%", "!", "<", ">",
];

pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let n = src[start..i]
                .parse::<f64>()
                .map_err(|_| ParseError::new("invalid number", start))?;
            out.push(Token {
                tok: Tok::Num(n),
                start,
                end: i,
            });
            continue;
        }
        if c == b'"' || c == b'\'' {
            let (s, end) = lex_string(src, i)?;
            out.push(Token {
                tok: Tok::Str(s),
                start,
                end,
            });
            i = end;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(src[start..i].to_string()),
                start,
                end: i,
            });
            continue;
        }
        match PUNCT.iter().find(|p| src[i..].starts_with(**p)) {
            Some(p) => {
                i += p.len();
                out.push(Token {
                    tok: Tok::Punct(p),
                    start,
                    end: i,
                });
            }
            None => {
                let ch = src[i..].chars().next().unwrap_or('?');
                return Err(ParseError::new(format!("unexpected character '{ch}'"), i));
            }
        }
    }
    out.push(Token {
        tok: Tok::Eof,
        start: src.len(),
        end: src.len(),
    });
    Ok(out)
}

fn lex_string(src: &str, start: usize) -> Result<(String, usize), ParseError> {
    let quote = src.as_bytes()[start] as char;
    let mut out = String::new();
    let mut chars = src[start + 1..].char_indices();
    while let Some((off, ch)) = chars.next() {
        match ch {
            c if c == quote => return Ok((out, start + 1 + off + 1)),
            '\\' => match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, c)) => out.push(c),
                None => break,
            },
            c => out.push(c),
        }
    }
    Err(ParseError::incomplete("unterminated text", start))
}
