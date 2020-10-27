#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::lexer::{Position, TokenKind, Token};

/// `ParseError` is an enum which represents errors encounted during parsing an expression
#[derive(Debug)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Error {
    info: String,
    position: Option<Position>,
}

impl Error {
    pub fn expect(expect_toks: &[TokenKind], tok: Token, context: String) -> Self {
        let info = format!(
            "expected {}, got '{}' in {}",
            if expect_toks.len() == 1 {
                format!(
                    "token '{}'",
                    expect_toks.first().map(TokenKind::to_string).unwrap()
                )
            } else {
                format!(
                    "one of {}",
                    expect_toks
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            format!(
                                "{}'{}'",
                                if i == 0 {
                                    ""
                                } else if i == expect_toks.len() - 1 {
                                    " or "
                                } else {
                                    ", "
                                },
                                t
                            )
                        })
                        .collect::<String>()
                )
            },
            tok,
            context,
        );
        Self::new(info, Some(tok.position))
    }
    pub fn unexpect(tok: Token, context: Option<String>) -> Self {
        let info = match context {
            Some(ctx) => format!("unexpected token '{}' {}", tok, ctx),
            None => format!("unexpected token '{}'", tok,),
        };
        Self::new(info, Some(tok.position))
    }
    pub fn new(info: String, position: Option<Position>) -> Self {
        Self { info, position }
    }
    pub fn abort() -> Self {
        Self {
            info: String::from("abort end"),
            position: None,
        }
    }
    pub fn position(&self) -> &Option<Position> {
        &self.position
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.position {
            Some(position) => write!(
                formatter,
                "{} at line {} column {}",
                self.info, position.line, position.col,
            ),
            None => write!(formatter, "{}", self.info,),
        }
    }
}