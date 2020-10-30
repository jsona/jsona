use crate::ast::Position;
use crate::lexer::{Token, TokenKind};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize)]
pub struct Error {
    pub info: String,
    pub position: Position,
}

impl Error {
    pub fn new(info: String, position: Position) -> Self {
        Self { info, position }
    }
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
        Self::new(info, tok.position)
    }
    pub fn unexpect(tok: Token, context: Option<String>) -> Self {
        let info = match context {
            Some(ctx) => format!("unexpected token '{}' in {}", tok, ctx),
            None => format!("unexpected token '{}'", tok,),
        };
        Self::new(info, tok.position)
    }
    pub fn abort() -> Self {
        Self {
            info: String::from("abort"),
            position: Position::default(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        if self.position.index == 0 {
            write!(formatter, "{}", self.info,)
        } else {
            write!(
                formatter,
                "{} at line {} column {}",
                self.info, self.position.line, self.position.col,
            )
        }
    }
}
