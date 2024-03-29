//! The JSONA abstract syntax tree module.

pub use jsona::util::mapper::{Mapper, Position, Range};
use serde::{Deserialize, Serialize};
use serde_json::{Number as JsonNumber, Value};
use std::{str::FromStr, string::String as StdString};

use jsona::dom::{self, DomNode, Node};
use jsona::error::ErrorObject;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Ast {
    Null(Null),
    Bool(Bool),
    Number(Number),
    String(String),
    Array(Array),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Null {
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Bool {
    pub value: bool,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Number {
    pub value: JsonNumber,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct String {
    pub value: StdString,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Array {
    pub items: Vec<Ast>,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Object {
    pub properties: Vec<Property>,
    pub annotations: Vec<Annotation>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Property {
    pub key: Key,
    pub value: Ast,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Annotation {
    pub key: Key,
    pub value: AnnotationValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Key {
    pub name: StdString,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnnotationValue {
    pub value: Value,
    pub range: Option<Range>,
}

impl FromStr for Ast {
    type Err = Vec<ErrorObject>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mapper = Mapper::new_utf16(s, false);
        match s.parse::<Node>() {
            Ok(value) => Ok(node_to_ast(&value, &mapper)),
            Err(error) => Err(error.to_error_objects(&mapper)),
        }
    }
}

impl From<Ast> for Node {
    fn from(ast: Ast) -> Self {
        match ast {
            Ast::Null(Null { annotations, .. }) => {
                dom::Null::new(from_annotations(annotations)).into()
            }
            Ast::Bool(Bool {
                value, annotations, ..
            }) => dom::Bool::new(value, from_annotations(annotations)).into(),
            Ast::Number(Number {
                value, annotations, ..
            }) => dom::Number::new(value, from_annotations(annotations)).into(),
            Ast::String(String {
                value, annotations, ..
            }) => dom::String::new(value, from_annotations(annotations)).into(),
            Ast::Array(Array {
                items, annotations, ..
            }) => {
                let items: Vec<Node> = items.into_iter().map(Node::from).collect();
                dom::Array::new(items, from_annotations(annotations)).into()
            }
            Ast::Object(Object {
                properties,
                annotations,
                ..
            }) => {
                let mut props = dom::Map::default();
                for prop in properties {
                    props.add(
                        dom::Key::property(prop.key.name),
                        Node::from(prop.value),
                        None,
                    );
                }
                dom::Object::new(props, from_annotations(annotations)).into()
            }
        }
    }
}

fn node_to_ast(value: &Node, mapper: &Mapper) -> Ast {
    let mut annotations: Vec<Annotation> = vec![];
    if let Some(value_annotations) = value.annotations() {
        for (key, value) in value_annotations.value().read().iter() {
            let key_range = key.mapper_range(mapper);
            let value_range = value.mapper_range(mapper);
            annotations.push({
                Annotation {
                    key: Key {
                        name: key.value().to_string(),
                        range: key_range,
                    },
                    value: AnnotationValue {
                        value: value.to_plain_json(),
                        range: value_range,
                    },
                }
            });
        }
    }
    let range = value.mapper_range(mapper);
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
                items: elements,
                annotations,
                range,
            })
        }
        Node::Object(v) => {
            let mut properties: Vec<Property> = vec![];
            for (key, value) in v.value().read().iter() {
                let range = key.mapper_range(mapper);
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

fn from_annotations(annotations: Vec<Annotation>) -> Option<dom::Annotations> {
    if annotations.is_empty() {
        return None;
    }
    let mut map = dom::Map::default();
    for anno in annotations {
        map.add(
            dom::Key::annotation(anno.key.name),
            serde_json::from_value(anno.value.value).unwrap(),
            None,
        )
    }
    Some(dom::Annotations::new(map))
}
