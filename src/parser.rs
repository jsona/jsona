use indexmap::IndexMap;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::lexer::{Lexer, Position, Token, TokenKind};
use crate::value::{Amap, Key};

/// `ParseError` is an enum which represents errors encounted during parsing an expression
#[derive(Debug)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct ParseError {
    info: String,
    position: Option<Position>,
}

impl ParseError {
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
            Some(ctx) => format!(
                "unexpected token '{}' {}",
                tok,
                ctx
            ),
            None => format!(
                "unexpected token '{}'",
                tok,
            ),
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

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.position {
            Some(position) => write!(
                formatter,
                "{} at line {} column {}",
                self.info,
                position.line(),
                position.col(),
            ),
            None => write!(formatter, "{}", self.info,),
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
    fn on_event(&mut self, event: Event, position: Position);
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser<T> {
    scanner: Lexer<T>,
    buf: Option<Token>,
}

fn sanitize_token(tok: Token) -> ParseResult<Token> {
    if let TokenKind::LexError(message) = tok.kind {
        return Err(ParseError::new(message, Some(tok.position)));
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
        return Err(ParseError::abort());
    }
    fn next_token(&mut self) -> ParseResult<Token> {
        if let Some(tok) = self.buf.take() {
            return Ok(tok);
        }
        if let Some(tok) = self.scanner.next() {
            return sanitize_token(tok);
        }
        Err(ParseError::abort())
    }
    pub fn parse<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        self.parse_annotaions(recv)?;
        self.parse_node(recv)?;
        let tok = self.peek_token()?;
        if let TokenKind::Eof = tok.kind {
            Ok(())
        } else {
            Err(ParseError::unexpect(tok, None))
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
                            return Err(ParseError::new(
                                format!("unexpect identifier \"{}\"", v),
                                Some(tok.position),
                            ))
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
            _ => return Err(ParseError::unexpect(tok, None)),
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
                        return Err(ParseError::unexpect(tok, None));
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
                        return Err(ParseError::unexpect(tok, None));
                    }
                }
                _ if tok.is_node() => {
                    if !allow_comma {
                        self.parse_node(recv)?;
                        no_elem = false;
                        allow_comma = true;
                    } else {
                        return Err(ParseError::expect(&[TokenKind::Comma], tok, "array".into()));
                    }
                }
                _ => return Err(ParseError::unexpect(tok, None)),
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
                        return Err(ParseError::unexpect(tok, None));
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
                        return Err(ParseError::unexpect(tok, None));
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
                                return Err(ParseError::unexpect(tok, None));
                            }
                        }
                        _ => {
                            return Err(ParseError::expect(
                                &[
                                    TokenKind::Identifier("identifier".into()),
                                    TokenKind::StringLiteral("stringliteral".into()),
                                ],
                                tok,
                                "annotation".into(),
                            ))
                        }
                    }
                }
                _ => return Err(ParseError::unexpect(tok, None)),
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
                annotations.insert(Key::new(key, tok.position), args);
            } else {
                return Err(ParseError::expect(
                    &[TokenKind::Identifier("identifer".into())],
                    tok,
                    "annotation".into(),
                ));
            }
        }
        if annotations.len() > 0 {
            recv.on_event(Event::Annotations(annotations), pos);
        }
        Ok(())
    }
    fn parse_annotation_args(&mut self) -> ParseResult<IndexMap<Key, String>> {
        let mut amap: IndexMap<Key, String> = IndexMap::new();
        let mut allow_comma = false;
        loop {
            let tok = self.peek_token()?;
            match &tok.kind {
                TokenKind::Comma => {
                    if allow_comma {
                        allow_comma = false;
                        self.next_token()?;
                    } else {
                        return Err(ParseError::unexpect(tok, None));
                    }
                }
                TokenKind::RightParen => {
                    self.next_token()?;
                    break;
                }
                TokenKind::Identifier(key) | TokenKind::StringLiteral(key) => {
                    if !allow_comma {
                        self.next_token()?;
                        let tok2 = self.peek_token()?;
                        if let TokenKind::Eq = tok2.kind {
                            self.next_token()?;
                            let tok3 = self.next_token()?;
                            if tok3.is_value() {
                                amap.insert(Key::new(key, tok.position), tok3.get_value().unwrap());
                                allow_comma = true;
                            } else {
                                return Err(ParseError::unexpect(
                                    tok3,
                                    Some("in annotattion args".into()),
                                ));
                            }
                        } else if let TokenKind::RightParen = tok2.kind {
                            amap.insert(Key::new("_", tok.position), key.into());
                            continue;
                        } else {
                            return Err(ParseError::expect(
                                &[TokenKind::Eq],
                                tok2,
                                "annotation args".into(),
                            ));
                        }
                    } else {
                        return Err(ParseError::expect(
                            &[TokenKind::Comma],
                            tok,
                            "annotation args".into(),
                        ));
                    }
                }
                _ => {
                    return Err(ParseError::unexpect(
                        tok,
                        Some("in annotattion args".into()),
                    ))
                }
            }
        }
        Ok(amap)
    }
}
