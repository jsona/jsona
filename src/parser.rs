use std::fmt;

use crate::scanner::{Scanner, Token, TokenKind, Position};
use crate::value::{Value, Amap};

pub struct Parser<T> {
    scanner: Scanner<T>,
    buf: Option<Token>,
}
/// `ParseError` is an enum which represents errors encounted during parsing an expression
#[derive(Debug)]
pub enum ParseError {
    /// When it expected a certain kind of token, but got another as part of something
    Expected {
        expected: Box<[TokenKind]>,
        found: Token,
        context: &'static str,
    },
    /// When a token is unexpected
    Unexpected {
        found: Token,
        message: Option<&'static str>,
    },
    /// When there is an abrupt end to the parsing
    AbruptEnd,
    /// Catch all General Error
    General {
        message: &'static str,
        position: Position,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected {
                expected,
                found,
                context,
            } => write!(
                f,
                "expected {}, got '{}' in {} at line {}, col {}",
                if expected.len() == 1 {
                    format!(
                        "token '{}'",
                        expected.first().map(TokenKind::to_string).unwrap()
                    )
                } else {
                    format!(
                        "one of {}",
                        expected
                            .iter()
                            .enumerate()
                            .map(|(i, t)| {
                                format!(
                                    "{}'{}'",
                                    if i == 0 {
                                        ""
                                    } else if i == expected.len() - 1 {
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
                found,
                context,
                found.pos.line(),
                found.pos.col(),
            ),
            Self::Unexpected { found, message } => write!(
                f,
                "unexpected token '{}'{} at line {}, col {}",
                found,
                if let Some(m) = message {
                    format!(", {}", m)
                } else {
                    String::new()
                },
                found.pos.line(),
                found.pos.col(),
            ),
            Self::AbruptEnd => f.write_str("abrupt end"),
            Self::General { message, position } => write!(
                f,
                "{} at line {}, col {}",
                message,
                position.line(),
                position.col(),
            ),
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

impl<T: Iterator<Item = char>> Parser<T> {
    pub fn new(input: T) -> Self {
        Self {
            scanner: Scanner::new(input),
            buf: None,
        }
    }
    pub fn parse(&mut self) -> ParseResult<(Value, Amap)> {
        let token = self.next_token();
        todo!()
    }
    fn peek_token(&mut self) -> Option<Token> {
        todo!()
    }
    fn next_token(&mut self) -> Option<Token> {
        todo!()
    }
    fn parse_array(&mut self) -> ParseResult<(Value, Amap)> {
        todo!()
    }
    fn parse_object(&mut self) -> ParseResult<(Value, Amap)> {
        todo!()
    }
}