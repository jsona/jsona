use indexmap::IndexMap;
use std::fmt;

use crate::lexer::{Lexer, Position, Token, TokenKind};
use crate::value::Amap;

/// `ParseError` is an enum which represents errors encounted during parsing an expression
#[derive(Debug)]
pub enum ParseError {
    /// When it expected a certain kind of token, but got another as part of something
    Expected {
        expected: Box<[TokenKind]>,
        token: Token,
        context: String,
    },
    /// When a token is unexpected
    Unexpected {
        token: Token,
        message: Option<String>,
    },
    /// Catch all General Error
    General { position: Position, message: String },
    /// When there is an abrupt end to the parsing
    AbruptEnd,
}

impl ParseError {
    pub fn position(&self) -> Option<Position> {
        match self {
            ParseError::Expected { token, .. } => Some(token.position.clone()),
            ParseError::Unexpected { token, .. } => Some(token.position.clone()),
            ParseError::General { position, .. } => Some(position.clone()),
            ParseError::AbruptEnd => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected {
                expected,
                token: found,
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
                found.position.line(),
                found.position.col(),
            ),
            Self::Unexpected {
                token: found,
                message,
            } => write!(
                f,
                "unexpected token '{}' {} at line {}, col {}",
                found,
                if let Some(m) = message {
                    format!(", {}", m)
                } else {
                    String::new()
                },
                found.position.line(),
                found.position.col(),
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

#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    ArrayStart,
    ArrayStop,
    ObjectStart,
    ObjectStop,
    Annotations(Amap),

    Null,
    Boolean(bool),
    String(String),
    Integer(i64),
    Float(f64),
}

pub trait EventReceiver {
    fn on_event(&mut self, ev: Event, pos: Position);
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser<T> {
    scanner: Lexer<T>,
    buf: Option<Token>,
}

fn sanitize_token(tok: Token) -> ParseResult<Token> {
    if let TokenKind::LexError(message) = tok.kind {
        return Err(ParseError::General {
            message,
            position: tok.position,
        });
    }
    return Ok(tok);
}

impl<T: Iterator<Item = char>> Parser<T> {
    pub fn new(input: T) -> Self {
        Self {
            scanner: Lexer::new(input),
            buf: None,
        }
    }
    fn peek_token(&mut self) -> ParseResult<Token> {
        if let Some(tok) = self.buf.clone() {
            return Ok(tok.clone());
        }
        if let Some(tok) = self.scanner.next() {
            let tok = sanitize_token(tok.clone())?;
            self.buf = Some(tok.clone());
            return Ok(tok);
        }
        return Err(ParseError::AbruptEnd);
    }
    fn next_token(&mut self) -> ParseResult<Token> {
        if let Some(tok) = self.buf.take() {
            return Ok(tok);
        }
        if let Some(tok) = self.scanner.next() {
            return sanitize_token(tok);
        }
        Err(ParseError::AbruptEnd)
    }
    pub fn parse<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        self.parse_annotaions(recv)?;
        self.parse_node(recv)?;
        let tok = self.peek_token()?;
        if let TokenKind::Eof = tok.kind {
            Ok(())
        } else {
            Err(ParseError::Unexpected {
                token: tok,
                message: None,
            })
        }
    }
    fn parse_node<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        let tok = self.next_token()?;
        match tok.kind {
            TokenKind::LeftBrace => {
                recv.on_event(Event::ObjectStart, tok.position);
                self.parse_object(recv)?;
            }
            TokenKind::LeftBracket => {
                recv.on_event(Event::ArrayStart, tok.position);
                self.parse_array(recv)?;
            }
            TokenKind::Identifier(v) => {
                let ev = {
                    match v.as_str() {
                        "true" => Event::Boolean(true),
                        "false" => Event::Boolean(false),
                        "null" => Event::Null,
                        _ => {
                            return Err(ParseError::General {
                                message: format!("unexpect identifier {}", v),
                                position: tok.position,
                            })
                        }
                    }
                };
                recv.on_event(ev, tok.position);
            }
            TokenKind::IntegerLiteral(i) => {
                recv.on_event(Event::Integer(i), tok.position);
            }
            TokenKind::FloatLiteral(f) => {
                recv.on_event(Event::Float(f), tok.position);
            }
            TokenKind::StringLiteral(s) => {
                recv.on_event(Event::String(s), tok.position);
            }
            _ => {
                return Err(ParseError::Unexpected {
                    message: None,
                    token: tok,
                })
            }
        };
        Ok(())
    }
    fn parse_array<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        self.parse_annotaions(recv)?;
        let mut allow_comma = false;
        let mut no_elem = true;
        loop {
            let tok = self.peek_token()?;
            match tok.kind {
                TokenKind::Comma => {
                    if allow_comma {
                        self.next_token()?;
                        allow_comma = false;
                    } else {
                        return Err(ParseError::Unexpected {
                            message: None,
                            token: tok,
                        });
                    }
                }
                TokenKind::RightBracket => {
                    recv.on_event(Event::ArrayStop, tok.position);
                    self.next_token()?;
                    break;
                }
                TokenKind::At => {
                    self.parse_annotaions(recv)?;
                    let allow_annotations = {
                        if !allow_comma {
                            true
                        } else {
                            if let TokenKind::RightBracket = self.peek_token()?.kind {
                                true
                            } else {
                                false
                            }
                        }
                    };
                    if !allow_annotations || no_elem {
                        return Err(ParseError::Unexpected {
                            message: None,
                            token: tok,
                        });
                    }
                }
                _ if tok.is_node() => {
                    if !allow_comma {
                        self.parse_node(recv)?;
                        no_elem = false;
                        allow_comma = true;
                    } else {
                        return Err(ParseError::Expected {
                            expected: Box::new([TokenKind::Comma]),
                            token: tok,
                            context: "array".into(),
                        });
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: None,
                        token: tok,
                    })
                }
            }
        }
        Ok(())
    }
    fn parse_object<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        self.parse_annotaions(recv)?;
        let mut allow_comma = false;
        let mut no_kv = true;
        loop {
            let tok = self.peek_token()?;
            match tok.kind {
                TokenKind::Comma => {
                    if allow_comma {
                        self.next_token()?;
                        allow_comma = false;
                    } else {
                        return Err(ParseError::Unexpected {
                            message: None,
                            token: tok,
                        });
                    }
                }
                TokenKind::RightBrace => {
                    recv.on_event(Event::ObjectStop, tok.position);
                    self.next_token()?;
                    break;
                }
                TokenKind::At => {
                    self.parse_annotaions(recv)?;
                    let allow_annotations = {
                        if !allow_comma {
                            true
                        } else {
                            if let TokenKind::RightBrace = self.peek_token()?.kind {
                                true
                            } else {
                                false
                            }
                        }
                    };
                    if !allow_annotations || no_kv {
                        return Err(ParseError::Unexpected {
                            message: None,
                            token: tok,
                        });
                    }
                }
                TokenKind::Identifier(..) | TokenKind::StringLiteral(..) => {
                    let tok = self.next_token()?;
                    let key = tok.get_value().unwrap();
                    recv.on_event(Event::String(key), tok.position);
                    let tok = self.peek_token()?;
                    match tok.kind {
                        TokenKind::Colon => {
                            self.next_token()?;
                            let tok_next = self.peek_token()?;
                            if tok_next.is_node() {
                                self.parse_node(recv)?;
                                no_kv = false;
                                allow_comma = true;
                            } else {
                                return Err(ParseError::Unexpected {
                                    message: None,
                                    token: tok,
                                });
                            }
                        }
                        _ => {
                            return Err(ParseError::Expected {
                                expected: Box::new([TokenKind::Identifier("identifer".into())]),
                                token: tok,
                                context: "annotation".into(),
                            });
                        }
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: None,
                        token: tok,
                    })
                }
            }
        }
        Ok(())
    }
    fn parse_annotaions<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        let mut annotations: Amap = IndexMap::new();
        let pos = self.peek_token()?.position;
        while let TokenKind::At = self.peek_token()?.kind {
            self.next_token()?;
            let tok = self.peek_token()?;

            if let TokenKind::Identifier(key) = tok.kind {
                self.next_token()?;
                let tok2 = self.peek_token()?;
                let args = {
                    if let TokenKind::LeftParen = tok2.kind {
                        self.next_token()?;
                        self.parse_annotation_args()?
                    } else {
                        IndexMap::new()
                    }
                };
                annotations.insert(key, args);
            } else {
                return Err(ParseError::Expected {
                    expected: Box::new([TokenKind::Identifier("identifer".into())]),
                    token: tok,
                    context: "annotation".into(),
                });
            }
        }
        if annotations.len() > 0 {
            recv.on_event(Event::Annotations(annotations), pos);
        }
        Ok(())
    }
    fn parse_annotation_args(&mut self) -> ParseResult<IndexMap<String, String>> {
        let mut amap: IndexMap<String, String> = IndexMap::new();
        let mut allow_comma = false;
        loop {
            let tok = self.peek_token()?;
            match &tok.kind {
                TokenKind::Comma => {
                    if allow_comma {
                        allow_comma = false;
                        self.next_token()?;
                    } else {
                        return Err(ParseError::Unexpected {
                            message: None,
                            token: tok,
                        });
                    }
                }
                TokenKind::RightParen => {
                    self.next_token()?;
                    break;
                }
                TokenKind::Identifier(key) | TokenKind::StringLiteral(key) => {
                    if !allow_comma {
                        self.next_token()?;
                        let tok = self.next_token()?;
                        if let TokenKind::Eq = tok.kind {
                            let tok = self.next_token()?;
                            if tok.is_value() {
                                amap.insert(key.to_owned(), tok.get_value().unwrap());
                                allow_comma = true;
                            } else {
                                return Err(ParseError::Unexpected {
                                    message: Some("in annotattion args".into()),
                                    token: tok,
                                });
                            }
                        } else {
                            return Err(ParseError::Expected {
                                expected: Box::new([TokenKind::Eq]),
                                token: tok,
                                context: "annotation args".into(),
                            });
                        }
                    } else {
                        return Err(ParseError::Expected {
                            expected: Box::new([TokenKind::Comma]),
                            token: tok,
                            context: "annotation args".into(),
                        });
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: Some("in annotattion args".into()),
                        token: tok,
                    })
                }
            }
        }
        Ok(amap)
    }
}
