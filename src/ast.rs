use indexmap::IndexMap;

use crate::lexer::Position;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};
use std::string;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde-support", serde(untagged))]
pub enum Ast {
    Null(Null),
    Boolean(Boolean),
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Array),
    Object(Object),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Null {
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Boolean {
    pub value: bool,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Integer {
    pub value: i64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Float {
    pub value: f64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct String {
    pub value: string::String,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Array {
    pub elements: Vec<Ast>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Object {
    pub properties: Vec<Property>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Property {
    pub key: string::String,
    pub position: Position,
    pub value: Ast,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
pub struct Annotation {
    pub name: string::String,
    pub position: Position,
    pub value: Option<Value>,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde-support", serde(untagged))]
pub enum Value {
    Null,
    Boolean(bool),
    Float(f64),
    Integer(i64),
    String(string::String),
    Array(Vec<Value>),
    Object(IndexMap<string::String, Value>),
}

macro_rules! define_is (
    ($name:ident, $yt:ident) => (
pub fn $name(&self) -> bool {
    match self {
        Ast::$yt($yt { .. }) => true,
        _ => false
    }
}
    );
);

impl Ast {
    define_is!(is_null, Null);
    define_is!(is_boolean, Boolean);
    define_is!(is_integer, Integer);
    define_is!(is_float, Float);
    define_is!(is_string, String);
    define_is!(is_array, Array);
    define_is!(is_object, Object);

    pub fn key(&self, key: &str) -> Option<&Self> {
        match self {
            Ast::Object(Object {
                properties: value, ..
            }) => value.iter().find(|p| p.key == key).map(|v| &v.value),
            Ast::Array(Array {
                elements: value, ..
            }) => {
                if let Ok(idx) = key.parse::<usize>() {
                    value.get(idx)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn retrive(&self, path: &[&str]) -> Option<&Self> {
        path.iter()
            .fold(Some(self), |v, &b| v.and_then(|v| v.key(b)))
    }
    pub fn get_annotations(&self) -> &Vec<Annotation> {
        match self {
            Ast::Null(Null { annotations, .. }) => annotations,
            Ast::Boolean(Boolean { annotations, .. }) => annotations,
            Ast::Integer(Integer { annotations, .. }) => annotations,
            Ast::Float(Float { annotations, .. }) => annotations,
            Ast::String(String { annotations, .. }) => annotations,
            Ast::Array(Array { annotations, .. }) => annotations,
            Ast::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Vec<Annotation> {
        match self {
            Ast::Null(Null { annotations, .. }) => annotations,
            Ast::Boolean(Boolean { annotations, .. }) => annotations,
            Ast::Integer(Integer { annotations, .. }) => annotations,
            Ast::Float(Float { annotations, .. }) => annotations,
            Ast::String(String { annotations, .. }) => annotations,
            Ast::Array(Array { annotations, .. }) => annotations,
            Ast::Object(Object { annotations, .. }) => annotations,
        }
    }
}

impl From<Ast> for Value {
    fn from(node: Ast) -> Self {
        match node {
            Ast::Null(..) => Value::Null,
            Ast::Boolean(Boolean { value, .. }) => Value::Boolean(value),
            Ast::Integer(Integer { value, .. }) => Value::Integer(value),
            Ast::Float(Float { value, .. }) => Value::Float(value),
            Ast::String(String { value, .. }) => Value::String(value),
            Ast::Array(Array {
                elements: value, ..
            }) => Value::Array(value.into_iter().map(|v| v.into()).collect()),
            Ast::Object(Object {
                properties: value, ..
            }) => Value::Object(
                value
                    .into_iter()
                    .map(|v| (v.key, v.value.into()))
                    .collect::<IndexMap<string::String, Value>>(),
            ),
        }
    }
}

macro_rules! define_value_is (
    ($name:ident, $yt:ident) => (
pub fn $name(&self) -> bool {
    match self {
        Value::$yt(..) => true,
        _ => false
    }
}
    );
);

macro_rules! define_value_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Value::$yt(ref v) => Some(v),
        _ => None
    }
}
    );
);

impl Value {
    pub fn is_null(&self) -> bool {
        match self {
            Value::Null => true,
            _ => false,
        }
    }
    define_value_is!(is_boolean, Boolean);
    define_value_is!(is_integer, Integer);
    define_value_is!(is_float, Float);
    define_value_is!(is_string, String);
    define_value_is!(is_array, Array);
    define_value_is!(is_object, Object);

    define_value_as_ref!(as_boolean, &bool, Boolean);
    define_value_as_ref!(as_integer, &i64, Integer);
    define_value_as_ref!(as_float, &f64, Float);
    define_value_as_ref!(as_string, &str, String);
    define_value_as_ref!(as_array, &Vec<Value>, Array);
    define_value_as_ref!(as_object, &IndexMap<string::String, Value>, Object);

    pub fn key(&self, key: &str) -> Option<&Self> {
        match self {
            Value::Object(value) => value.get(key),
            Value::Array(value) => {
                if let Ok(idx) = key.parse::<usize>() {
                    value.get(idx)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn retrive(&self, path: &[&str]) -> Option<&Self> {
        path.iter()
            .fold(Some(self), |v, &b| v.and_then(|v| v.key(b)))
    }
}
