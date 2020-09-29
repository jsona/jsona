use indexmap::IndexMap;
use std::fmt;

use crate::lexer::{Lexer, Position, Token, TokenKind};
use crate::value::{Amap, Value};

/// `ParseError` is an enum which represents errors encounted during parsing an expression
#[derive(Debug)]
pub enum ParseError {
    /// When it expected a certain kind of token, but got another as part of something
    Expected {
        expected: Box<[TokenKind]>,
        found: Token,
        context: String,
    },
    /// When a token is unexpected
    Unexpected {
        found: Token,
        message: Option<String>,
    },
    /// When there is an abrupt end to the parsing
    AbruptEnd,
    /// Catch all General Error
    General { message: String, position: Position },
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
                "unexpected token '{}' {} at line {}, col {}",
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

pub struct Parser<T> {
    scanner: Lexer<T>,
    buf: Option<Token>,
}

fn sanitize_token(tok: Token) -> ParseResult<Token> {
    if let TokenKind::LexError(message) = tok.kind {
        return Err(ParseError::General {
            message,
            position: tok.pos,
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
    pub fn parse(&mut self) -> ParseResult<(Value, Option<Amap>)> {
        let annotations = self.parse_annotaions()?;
        let value = self.parse_node()?;
        Ok((value, annotations))
    }
    fn parse_node(&mut self) -> ParseResult<Value> {
        let tok = self.next_token()?;
        match tok.kind {
            TokenKind::LeftBrace => {
                return self.parse_object();
            }
            TokenKind::LeftBracket => {
                return self.parse_array();
            }
            TokenKind::Identifier(v) => match v.as_str() {
                "true" => return Ok(Value::Boolean(true, None)),
                "false" => return Ok(Value::Boolean(false, None)),
                "null" => return Ok(Value::Null(None)),
                _ => {
                    return Err(ParseError::General {
                        message: format!("unexpect identifier {}", v),
                        position: tok.pos,
                    })
                }
            },
            TokenKind::IntegerLiteral(i) => return Ok(Value::Integer(i, None)),
            TokenKind::FloatLiteral(f) => return Ok(Value::Float(f, None)),
            TokenKind::StringLiteral(s) => return Ok(Value::String(s, None)),
            _ => {
                return Err(ParseError::Unexpected {
                    message: None,
                    found: tok,
                })
            }
        }
    }
    fn parse_array(&mut self) -> ParseResult<Value> {
        let annotations = self.parse_annotaions()?;
        let mut elems: Vec<Value> = Vec::new();
        let mut allow_comma = false;
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
                            found: tok,
                        });
                    }
                }
                TokenKind::RightBracket => {
                    self.next_token()?;
                    break;
                }
                TokenKind::At => {
                    let annotations = self.parse_annotaions()?;
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
                    if !allow_annotations || elems.len() == 0 {
                        return Err(ParseError::Unexpected {
                            message: None,
                            found: tok,
                        });
                    }
                    elems.last_mut().map(|v| v.set_annotiaons(annotations));
                }
                _ if tok.is_node() => {
                    if !allow_comma {
                        let elem = self.parse_node()?;
                        allow_comma = true;
                        elems.push(elem);
                    } else {
                        return Err(ParseError::Expected {
                            expected: Box::new([TokenKind::Comma]),
                            found: tok,
                            context: "array".into(),
                        });
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: None,
                        found: tok,
                    })
                }
            }
        }
        Ok(Value::Array(elems, annotations))
    }
    fn parse_object(&mut self) -> ParseResult<Value> {
        let annotations = self.parse_annotaions()?;
        let mut kvs: IndexMap<String, Value> = IndexMap::new();
        let mut allow_comma = false;
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
                            found: tok,
                        });
                    }
                }
                TokenKind::RightBrace => {
                    self.next_token()?;
                    break;
                }
                TokenKind::At => {
                    let annotations = self.parse_annotaions()?;
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
                    if !allow_annotations || kvs.len() == 0 {
                        return Err(ParseError::Unexpected {
                            message: None,
                            found: tok,
                        });
                    }
                    kvs.get_index_mut(kvs.len() - 1)
                        .map(|(_, v)| v.set_annotiaons(annotations));
                }
                TokenKind::Identifier(..) | TokenKind::StringLiteral(..) => {
                    let tok = self.next_token()?;
                    let key = tok.get_value().unwrap();
                    let tok = self.peek_token()?;
                    match tok.kind {
                        TokenKind::Colon => {
                            self.next_token()?;
                            let tok1 = self.peek_token()?;
                            if tok1.is_node() {
                                let value = self.parse_node()?;
                                kvs.insert(key, value);
                                allow_comma = true;
                            } else {
                                return Err(ParseError::Unexpected {
                                    message: None,
                                    found: tok,
                                });
                            }
                        }
                        _ => {
                            return Err(ParseError::Expected {
                                expected: Box::new([TokenKind::Identifier("identifer".into())]),
                                found: tok,
                                context: "annotation".into(),
                            });
                        }
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: None,
                        found: tok,
                    })
                }
            }
        }
        Ok(Value::Object(kvs, annotations))
    }
    fn parse_annotaions(&mut self) -> ParseResult<Option<Amap>> {
        let mut annotations: Amap = IndexMap::new();
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
                        Vec::new()
                    }
                };
                annotations.insert(key, args);
            } else {
                return Err(ParseError::Expected {
                    expected: Box::new([TokenKind::Identifier("identifer".into())]),
                    found: tok,
                    context: "annotation".into(),
                });
            }
        }
        if annotations.len() > 0 {
            Ok(Some(annotations))
        } else {
            Ok(None)
        }
    }
    fn parse_annotation_args(&mut self) -> ParseResult<Vec<String>> {
        let mut result: Vec<String> = Vec::new();
        let mut allow_comma = false;
        loop {
            let tok = self.peek_token()?;
            match tok.kind {
                TokenKind::Comma => {
                    if allow_comma {
                        allow_comma = false;
                        self.next_token()?;
                    } else {
                        return Err(ParseError::Unexpected {
                            message: None,
                            found: tok,
                        });
                    }
                }
                TokenKind::RightParen => {
                    self.next_token()?;
                    break;
                }
                _ if tok.is_value() => {
                    if !allow_comma {
                        let tok = self.next_token()?;
                        let v = tok.get_value().unwrap();
                        allow_comma = true;
                        result.push(v);
                    } else {
                        return Err(ParseError::Expected {
                            expected: Box::new([TokenKind::Comma]),
                            found: tok,
                            context: "annotation args".into(),
                        });
                    }
                }
                _ => {
                    return Err(ParseError::Unexpected {
                        message: Some("in annotattion args".into()),
                        found: tok,
                    })
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    macro_rules! map(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = IndexMap::new();
                $(
                    m.insert($key, $value);
                )+
                m
            }
        };
    );

    #[test]
    fn test_parse() {
        let s = r##"
@abc
@def(a, b)
/*
multi-line comments
*/

{
    a: null,
    b: 'say "hello"',
    c: true,
    m: "it's awesome",
    h: -3.13,
    d: [ @array
        "abc", @upper
        "def",
    ],
    o: { a:3, b: 4 },
    // This is comments
    g: { @object
        a: 3,
        b: 4,
        c: 5,
    },
    x: 0x1b,
    y: 3.2 @optional @xxg(a, b)
}
        "##;
        let mut parser = Parser::new(s.chars());
        let result = parser.parse().unwrap();
        let value = Value::Object(
            map! {
                "a".into() => Value::Null(None),
                "b".into() => Value::String(r#"say "hello""#.into(), None),
                "c".into() => Value::Boolean(true, None),
                "m".into() => Value::String(r#"it's awesome"#.into(), None),
                "h".into() => Value::Float(-3.13, None),
                "d".into() => Value::Array(
                    vec![
                        Value::String("abc".into(), Some(map!{ "upper".into() => vec![] })),
                        Value::String("def".into(), None),
                    ],
                    Some(map!{ "array".into() => vec![] })
                ),
                "o".into() => Value::Object(map!{
                    "a".into() => Value::Integer(3, None),
                    "b".into() => Value::Integer(4, None)
                }, None),
                "g".into() => Value::Object(
                    map!{
                        "a".into() => Value::Integer(3, None),
                        "b".into() => Value::Integer(4, None),
                        "c".into() => Value::Integer(5, None)
                    },
                    Some(map!{ "object".into() => vec![] })
                ),
                "x".into() => Value::Integer(27, None),
                "y".into() => Value::Float(
                    3.2,
                    Some(map!{ "optional".into() => vec![],  "xxg".into() => vec!["a".into(), "b".into()] })
                )
            },
            None,
        );
        let annotations: Amap = map! {
            "abc".into() => vec![],
            "def".into() => vec!["a".into(), "b".into()]
        };
        assert_eq!(value, result.0);
        assert_eq!(Some(annotations), result.1);
    }
}
