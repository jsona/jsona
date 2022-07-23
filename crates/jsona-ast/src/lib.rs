mod mapper;

pub use crate::mapper::{Mapper, Position, Range};

use serde::{Deserialize, Serialize};
use serde_json::{Number as JsonNumber, Value};
use std::{str::FromStr, string::String as StdString};

use jsona::dom::error::{Error as DomError, ParseError};
use jsona::dom::{DomNode, Node};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Ast {
    Null(Null),
    Bool(Bool),
    Number(Number),
    String(String),
    Array(Array),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Null {
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Bool {
    pub value: bool,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Number {
    pub value: JsonNumber,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct String {
    pub value: StdString,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Array {
    pub elements: Vec<Ast>,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Object {
    pub properties: Vec<Property>,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Property {
    pub key: Key,
    pub value: Ast,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Annotation {
    pub key: Key,
    pub value: AnnoationValue,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Key {
    pub name: StdString,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AnnoationValue {
    pub value: Value,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Error {
    pub kind: StdString,
    pub message: StdString,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorKind {
    InvalidSyntax,
    UnexpectedSyntax,
    InvalidEscapeSequence,
}

impl Error {
    pub fn new(kind: &str, message: &str, range: Option<Range>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            range,
        }
    }
}

impl FromStr for Ast {
    type Err = Vec<Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mapper = Mapper::new_utf16(s, false);
        let mut ast_errors: Vec<Error> = vec![];

        match s.parse::<Node>() {
            Ok(value) => return Ok(node_to_ast(&value, &mapper)),
            Err(error) => match error {
                ParseError::InvalidSyntax { errors } => {
                    for err in errors.into_iter() {
                        let message = err.to_string();
                        let range = mapper.range(err.range);
                        ast_errors.push(Error::new("InvalidSyntax", &message, range));
                    }
                }
                ParseError::InvalidDom { errors } => {
                    for err in errors.into_iter() {
                        let message = err.to_string();
                        match err {
                            DomError::ConflictingKeys { key, other } => {
                                let range = dom_range(&key, &mapper);
                                ast_errors.push(Error::new("ConflictingKeys", &message, range));
                                let range = dom_range(&other, &mapper);
                                ast_errors.push(Error::new("ConflictingKeys", &message, range));
                            }
                            DomError::UnexpectedSyntax { syntax } => {
                                let range = mapper.range(syntax.text_range());
                                ast_errors.push(Error::new("UnexpectedSyntax", &message, range));
                            }
                            DomError::InvalidEscapeSequence { syntax } => {
                                let range = mapper.range(syntax.text_range());
                                ast_errors.push(Error::new(
                                    "InvalidEscapeSequence",
                                    &message,
                                    range,
                                ));
                            }
                        }
                    }
                }
            },
        }
        Err(ast_errors)
    }
}

fn node_to_ast(value: &Node, mapper: &Mapper) -> Ast {
    let mut annotations: Vec<Annotation> = vec![];
    if let Some(annos) = value.annotations() {
        for (key, value) in annos.value().read().iter() {
            let key_range = dom_range(key, mapper);
            let value_range = dom_range(value, mapper);
            annotations.push({
                Annotation {
                    key: Key {
                        name: key.value().to_string(),
                        range: key_range,
                    },
                    value: AnnoationValue {
                        value: value.to_plain_json(),
                        range: value_range,
                    },
                }
            });
        }
    }
    let range = dom_range(value, mapper);
    match value {
        Node::Null(_) => Ast::Null(Null { annotations, range }),
        Node::Bool(v) => Ast::Bool(Bool {
            value: v.value(),
            annotations,
            range,
        }),
        Node::Number(v) => Ast::Number(Number {
            value: v.value().clone(),
            annotations,
            range,
        }),
        Node::String(v) => Ast::String(String {
            value: v.value().to_string(),
            annotations,
            range,
        }),
        Node::Array(v) => {
            let elements = v
                .value()
                .read()
                .iter()
                .map(|v| node_to_ast(v, mapper))
                .collect();
            Ast::Array(Array {
                elements,
                annotations,
                range,
            })
        }
        Node::Object(v) => {
            let mut properties: Vec<Property> = vec![];
            for (key, value) in v.value().read().iter() {
                let range = dom_range(key, mapper);
                properties.push({
                    Property {
                        key: Key {
                            name: key.value().to_string(),
                            range,
                        },
                        value: node_to_ast(value, mapper),
                    }
                });
            }
            Ast::Object(Object {
                properties,
                annotations,
                range,
            })
        }
    }
}

fn dom_range<T: DomNode>(node: &T, mapper: &Mapper) -> Option<Range> {
    node.syntax()
        .and_then(|syntax| mapper.range(syntax.text_range()))
}
