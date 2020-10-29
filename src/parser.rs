use crate::error::Error;

use crate::ast::Position;
use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    ArrayStart,
    ArrayStop,
    ObjectStart,
    ObjectStop,
    AnnotationStart(String),
    AnnotationEnd,
    Null,
    Boolean(bool),
    String(String),
    Integer(i64),
    Float(f64),
}

pub trait EventReceiver {
    fn on_event(&mut self, event: Event, position: Position);
}

pub type ParseResult<T> = Result<T, Error>;

pub struct Parser<T> {
    scanner: Lexer<T>,
    buf: Option<Token>,
    annotation_scope: bool,
}

fn sanitize_token(tok: Token) -> ParseResult<Token> {
    if let TokenKind::LexError(message) = tok.kind {
        return Err(Error::new(message, tok.position));
    }
    return Ok(tok);
}

impl<T: Iterator<Item = char>> Parser<T> {
    pub fn new(input: T) -> Self {
        Self {
            scanner: Lexer::new(input),
            buf: None,
            annotation_scope: false,
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
        return Err(Error::abort());
    }
    fn next_token(&mut self) -> ParseResult<Token> {
        if let Some(tok) = self.buf.take() {
            return Ok(tok);
        }
        if let Some(tok) = self.scanner.next() {
            return sanitize_token(tok);
        }
        Err(Error::abort())
    }
    pub fn parse<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        self.parse_node(recv)?;
        let tok = self.peek_token()?;
        if let TokenKind::Eof = tok.kind {
            Ok(())
        } else if self.annotation_scope && TokenKind::RightBrace == tok.kind {
            Ok(())
        } else {
            Err(Error::unexpect(tok, None))
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
                            return Err(Error::new(
                                format!("unexpect identifier \"{}\"", v),
                                tok.position,
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
            _ => return Err(Error::unexpect(tok, None)),
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
                        return Err(Error::unexpect(tok, None));
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
                        return Err(Error::unexpect(tok, None));
                    }
                }
                _ if tok.is_node() => {
                    if !allow_comma {
                        self.parse_node(recv)?;
                        no_elem = false;
                        allow_comma = true;
                    } else {
                        return Err(Error::expect(&[TokenKind::Comma], tok, "array".into()));
                    }
                }
                _ => return Err(Error::unexpect(tok, None)),
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
                        return Err(Error::unexpect(tok, None));
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
                        return Err(Error::unexpect(tok, None));
                    }
                }
                TokenKind::Identifier(..)
                | TokenKind::StringLiteral(..)
                | TokenKind::IntegerLiteral(..) => {
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
                                return Err(Error::unexpect(tok, None));
                            }
                        }
                        _ => {
                            return Err(Error::expect(
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
                _ => return Err(Error::unexpect(tok, None)),
            }
        }
        Ok(())
    }
    fn parse_annotaions<R: EventReceiver>(&mut self, recv: &mut R) -> ParseResult<()> {
        let tok = self.peek_token()?;
        if let TokenKind::At = tok.kind {
            if self.annotation_scope {
                return Err(Error::unexpect(tok, Some("in annotation value".into())));
            }
            self.next_token()?;
            let tok2 = self.peek_token()?;
            if let TokenKind::Identifier(key) = tok2.kind {
                self.next_token()?;
                recv.on_event(Event::AnnotationStart(key), tok2.position);
                let tok3 = self.peek_token()?;
                if let TokenKind::LeftParen = tok3.kind {
                    self.next_token()?;
                    self.annotation_scope = true;
                    self.parse_node(recv)?;
                    let tok4 = self.next_token()?;
                    self.annotation_scope = false;
                    recv.on_event(Event::AnnotationEnd, tok4.position);
                    self.parse_annotaions(recv)?;
                } else {
                    recv.on_event(Event::Null, tok2.position);
                    recv.on_event(Event::AnnotationEnd, tok2.position);
                    self.parse_annotaions(recv)?;
                }
            } else {
                return Err(Error::expect(
                    &[TokenKind::Identifier("identifer".into())],
                    tok2,
                    "annotation".into(),
                ));
            }
        }
        Ok(())
    }
}
